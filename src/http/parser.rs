use async_std::io::Read;

use super::rule::{HTTPMessage, HeaderField, FieldName, FieldValue, FieldContent, RequestLine, Method, OriginForm, HTTPVersion, AbsolutePath, Query, Segment};
use super::flatten::{flatten, HTTPRequest};
use crate::peekable_bufreader::PeekableBufReader;

const HTAB: u8 = 0x09;
const SPACE: u8 = 0x20;
const PERCENT: u8 = 0x25;
const SLASH: u8 = 0x2F;
const DOT: u8 = 0x2E;
const COLON: u8 = 0x3A;
const QUESTION_MARK: u8 = 0x3F;
const ATSIGN: u8 = 0x40;

enum ErrorType {
    Missing,
    Malformed,
}

pub struct Parser<T>
    where T: Read + Unpin {
    source: PeekableBufReader<T>,
}

impl<T> Parser<T>
    where T: Read + Unpin {
    pub fn new(source: PeekableBufReader<T>) -> Self {
        Self {
            source,
        }
    }

    pub async fn parse(self) -> Option<HTTPRequest> {
        flatten(self.http_message().await?)
    }

    /*
    * RFC 7230, Page 19
    */
    async fn http_message(mut self) -> Option<HTTPMessage> {
        let start_line = self.start_line().await?;
        let mut header_fields = Vec::new();
        loop {
            if let Some(_) = self.consume_carriage_return().await {
                break;
            }
            header_fields.push(self.header_field().await?);
            self.consume_carriage_return().await?;
        }
        // GET Requests don't have a message body, and we only really deal with GET requests.
        // There's no need to examine the headers and attempt to read a message body.
        Some(HTTPMessage {
            request_line: start_line,
            header_fields,
        })
    }

    /*
    * RFC 7230, Page 23
    */
    async fn header_field(&mut self) -> Option<HeaderField> {
        let name = self.field_name().await?;
        self.consume_char(&COLON).await?;
        self.consume_optional_whitespace().await;
        let value = self.field_value().await?;
        self.consume_optional_whitespace().await;
        Some(HeaderField {
            name,
            value,
        })
    }

    /*
    * RFC 7230, Page 23
    */
    async fn field_name(&mut self) -> Option<FieldName> {
        Some(FieldName {
            lexeme: self.logical_token().await?,
        })
    }

    /*
    * RFC 7230, Page 23
    * obs-fold is deprecated except within message/http media. This isn't going to come up for us,
    * so we deviate from the grammar slightly.
    */
    async fn field_value(&mut self) -> Option<FieldValue> {
        let mut content = Vec::new();
        loop {
            // Look, no obs-fold!
            if let Some(value) = self.field_content().await {
                // Let's do some pre-emptive flattening here.
                content.push(value.first_char);
                if let Some(second_char) = value.second_char {
                    content.push(SPACE);
                    content.push(second_char);
                }
            } else {
                break;
            }
        }
        Some(FieldValue {
            content,
        })
    }

    /*
    * RFC 7230, Page 23
    */
    async fn field_content(&mut self) -> Option<FieldContent> {
        let first_char = self.field_vchar().await?;
        let second_char = if let Some(_) = self.consume_required_whitespace().await {
            Some(self.field_vchar().await?)
        } else {
            None
        };
        Some(FieldContent{
            first_char,
            second_char,
        })
    }

    async fn field_vchar(&mut self) -> Option<u8> {
        let next_char = self.source.peek().await?;
        if Self::is_visible_char(next_char) || Self::is_obs_text_char(next_char) {
            return self.source.next().await;
        }
        None
    }

    /*
    * RFC 7230, Page 21
    * This is a server, so the start-line is exclusively a request-line.
    */
    async fn start_line(&mut self) -> Option<RequestLine> {
        self.request_line().await
    }
    
    /*
    * RFC 7230, Page 21
    */
    async fn request_line(&mut self) -> Option<RequestLine> {
        let method = self.method().await?;
        self.consume_char(&SPACE).await?;
        let request_target = self.request_target().await?;
        self.consume_char(&SPACE).await?;
        let http_version = self.http_version().await?;
        self.consume_carriage_return().await?;
        Some(RequestLine {
            method,
            request_target,
            http_version,
        })
    }

    /*
    * RFC 7230, Page 41
    * We only serve some static content; therefore we only need support origin-form.
    */
    async fn request_target(&mut self) -> Option<OriginForm> {
        self.origin_form().await
    }

    /*
    * RFC 7230, Page 42
    */
    async fn origin_form(&mut self) -> Option<OriginForm> {
        let absolute_path = self.absolute_path().await?;
        let query = if let Some(_) = self.consume_char(&QUESTION_MARK).await {
            Some(self.query().await?)
        } else {
            None
        };
        Some(OriginForm {
            absolute_path,
            query,
        })
    }

    /*
    * RFC 7230, Page 16
    */
    async fn absolute_path(&mut self) -> Option<AbsolutePath> {
        let mut segments = Vec::new();
        self.consume_char(&SLASH).await?;
        segments.push(self.segment().await?);
        loop {
            if let None = self.consume_char(&SLASH).await {
                break;
            }
            if let Some(segment) = self.segment().await {
                segments.push(segment);
            } else {
                return None;
            }
        }
        Some(AbsolutePath {
            segments,
        })
    }

    /*
    * RFC 3986, Page 23
    */
    async fn segment(&mut self) -> Option<Segment> {
        let mut segment = Vec::new();
        while self.source.peek().await.is_some() {
            match self.consume_path_character().await {
                Ok(character) => segment.push(character as char),
                Err(ErrorType::Missing) => break,
                Err(ErrorType::Malformed) => return None,
            }
        }
        Some(Segment{
            lexeme: segment.into_iter().collect(),
        })
    }

    /*
    * RFC 3986, Page 50
    */
    async fn query(&mut self) -> Option<Query> {
        let mut query = Vec::new();
        while self.source.peek().await.is_some() {
            match self.consume_query_character().await {
                Ok(character) => query.push(character as char),
                Err(ErrorType::Missing) => break,
                Err(ErrorType::Malformed) => return None,
            }
        }
        Some(Query{
            lexeme: query.into_iter().collect(),
        })
    }

    /*
    * RFC 7230, Page 14
    */
    async fn http_version(&mut self) -> Option<HTTPVersion> {
        self.consume_logical_token("HTTP").await?;
        self.consume_char(&SLASH).await?;
        let major = Self::ascii_digit_to_value(&self.consume_digit().await?);
        self.consume_char(&DOT).await?;
        let minor = Self::ascii_digit_to_value(&self.consume_digit().await?);
        Some(HTTPVersion{
            major,
            minor,
        })
    }
    
    /*
    * RFC 7230, Page 21
    */
    async fn method(&mut self) -> Option<Method> {
        Method::from_string(&self.logical_token().await?)
    }

    /*
    * RFC 7230, Page 27
    */
    async fn logical_token(&mut self) -> Option<String> {
        let mut logical_token = Vec::new();
        if !Self::is_logical_token_char(self.source.peek().await?) {
            return None;
        }
        while self.source.peek().await.is_some() && Self::is_logical_token_char(self.source.peek().await.unwrap()) {
            logical_token.push(self.source.next().await.unwrap() as char);
        }
        Some(logical_token.into_iter().collect())
    }

    async fn consume_char(&mut self, character: &u8) -> Option<u8> {
        let next_char = self.source.peek().await?;
        if *next_char == *character {
            return self.source.next().await;
        }
        None
    }
    
    async fn consume_logical_token(&mut self, value: &str) -> Option<String> {
        let logical_token = self.logical_token().await?;
        if logical_token == value {
            return Some(logical_token);
        }
        None
    }

    /*
    * RFC 3986, Page 23
    */
    async fn consume_path_character(&mut self) -> Result<u8, ErrorType> {
        match self.consume_unreserved_character().await {
            Some(character) => return Ok(character),
            _ => {}
        }
        match self.consume_sub_delim_character().await {
            Some(character) => return Ok(character),
            _ => {}
        }
        match self.consume_char(&COLON).await {
            Some(character) => return Ok(character),
            _ => {}
        }
        match self.consume_char(&ATSIGN).await {
            Some(character) => return Ok(character),
            _ => {}
        }
        self.consume_percent_encoded().await
    }

    /*
    * RFC 3986, Page 50
    */
    async fn consume_query_character(&mut self) -> Result<u8, ErrorType> {
        match self.consume_unreserved_character().await {
            Some(character) => return Ok(character),
            _ => {}
        }
        match self.consume_sub_delim_character().await {
            Some(character) => return Ok(character),
            _ => {}
        }
        match self.consume_char(&COLON).await {
            Some(character) => return Ok(character),
            _ => {}
        }
        match self.consume_char(&ATSIGN).await {
            Some(character) => return Ok(character),
            _ => {}
        }
        match self.consume_char(&SLASH).await {
            Some(character) => return Ok(character),
            _ => {}
        }
        match self.consume_char(&QUESTION_MARK).await {
            Some(character) => return Ok(character),
            _ => {}
        }
        self.consume_percent_encoded().await
    }

    /*
    * RFC 5234, Page 5
    */
    async fn consume_carriage_return(&mut self) -> Option<()> {
        self.consume_char(&0x0D).await?;
        self.consume_char(&0x0A).await?;
        Some(())
    }

    async fn consume_optional_whitespace(&mut self) {
        loop {
            if let None = self.consume_char(&SPACE).await {
                if let None = self.consume_char(&HTAB).await {
                    break;
                }
            }
        }
    }

    async fn consume_required_whitespace(&mut self) -> Option<()> {
        if let None = self.consume_char(&SPACE).await {
            if let None = self.consume_char(&HTAB).await {
                return None;
            }
        }
        loop {
            if let None = self.consume_char(&SPACE).await {
                if let None = self.consume_char(&HTAB).await {
                    break;
                }
            }
        }
        Some(())
    }

    /*
    * RFC 5234, Page 14
    */
    async fn consume_digit(&mut self) -> Option<u8> {
        let next_char = self.source.peek().await?;
        if Self::is_digit_char(next_char) {
            return self.source.next().await
        }
        None
    }

    async fn consume_unreserved_character(&mut self) -> Option<u8> {
        let next_char = self.source.peek().await?;
        if Self::is_unreserved_char(next_char) {
            return self.source.next().await
        }
        None
    }

    async fn consume_sub_delim_character(&mut self) -> Option<u8> {
        let next_char = self.source.peek().await?;
        if Self::is_sub_delim_char(next_char) {
            return self.source.next().await
        }
        None
    }

    async fn consume_percent_encoded(&mut self) -> Result<u8, ErrorType> {
        self.consume_char(&PERCENT).await.ok_or(ErrorType::Missing)?;
        let high_word = self.consume_hex_digit().await.ok_or(ErrorType::Malformed)?;
        let low_word = self.consume_hex_digit().await.ok_or(ErrorType::Malformed)?;
        Self::hex_digits_to_byte(high_word, low_word).ok_or(ErrorType::Malformed)
    }

    async fn consume_hex_digit(&mut self) -> Option<u8> {
        let next_char = self.source.peek().await?;
        if Self::is_hex_digit_char(next_char) {
            return self.source.next().await
        }
        None
    }

    fn ascii_digit_to_value(character: &u8) -> u32 {
        *character as u32 - 0x30
    }

    fn hex_digits_to_byte(high_word: u8, low_word: u8) -> Option<u8> {
        u8::from_str_radix(&((high_word as char).to_string() + &(low_word as char).to_string()), 16).ok()
    }

    /*
    * RFC 7230, Page 27
    */
    fn is_logical_token_char(character: &u8) -> bool {
        *character == 0x21 || // !
        *character == 0x23 || // #
        *character == 0x24 || // $
        *character == 0x25 || // %
        *character == 0x26 || // &
        *character == 0x27 || // '
        *character == 0x2A || // *
        *character == 0x2B || // +
        *character == 0x2D || // -
        *character == 0x2E || // .
        *character == 0x5E || // ^
        *character == 0x5F || // _
        *character == 0x60 || // `
        *character == 0x7C || // |
        *character == 0x7E || // ~
        Self::is_digit_char(character) ||
        Self::is_alpha_char(character)
    }

    /*
    * RFC 3986, Page 13
    */
    fn is_unreserved_char(character: &u8) -> bool {
        Self::is_alpha_char(character) ||
        Self::is_digit_char(character) ||
        *character == 0x2D || // -
        *character == 0x2E || // .
        *character == 0x5F || // _
        *character == 0x7E    // ~
    }

    /*
    * RFC 3986, Page 13
    */
    fn is_sub_delim_char(character: &u8) -> bool {
        *character == 0x21 || // !
        *character == 0x24 || // $
        *character == 0x26 || // &
        *character == 0x27 || // '
        *character == 0x28 || // (
        *character == 0x29 || // )
        *character == 0x2A || // *
        *character == 0x2B || // +
        *character == 0x2C || // ,
        *character == 0x3B || // ;
        *character == 0x3D    // =
    }

    /*
    * RFC 5234, Page 13
    */
    fn is_alpha_char(character: &u8) -> bool {
        (*character >= 0x41 && *character <= 0x5A) || (*character >= 0x61 && *character <= 0x7A)
    }

    /*
    * RFC 5234, Page 14
    */
    fn is_digit_char(character: &u8) -> bool {
        *character >= 0x30 && *character <= 0x39
    }

    /*
    * RFC 5234, Page 14
    */
    fn is_hex_digit_char(character: &u8) -> bool {
        Self::is_digit_char(character) ||
        *character == 0x41 || // A
        *character == 0x42 || // B
        *character == 0x43 || // C
        *character == 0x44 || // D
        *character == 0x45 || // E
        *character == 0x46 || // F
        *character == 0x61 || // a
        *character == 0x62 || // b
        *character == 0x63 || // c
        *character == 0x64 || // d
        *character == 0x65 || // e
        *character == 0x66    // f
    }

    /*
    * RFC 5234, Page 14
    */
    fn is_visible_char(character: &u8) -> bool {
        *character >= 0x21 && *character <= 0x7E
    }

    fn is_obs_text_char(character: &u8) -> bool {
        *character >= 0x80
    }
}
