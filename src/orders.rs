use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GameCode(String);

impl GameCode {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GameCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Turn(u32);

impl Turn {
    pub fn number(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderFileHeader {
    pub game: GameCode,
    pub turn: Turn,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Email(String);

impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerId(u64);

impl PlayerId {
    pub fn number(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FactionId(u64);

impl FactionId {
    pub fn number(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum OrderFileOwner {
    Email(Email),
    Player(PlayerId),
    Faction(FactionId),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AuthenticationToken(String);

impl AuthenticationToken {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderFilePreamble {
    pub header: OrderFileHeader,
    pub owner: OrderFileOwner,
    pub token: AuthenticationToken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    filename: String,
    line: usize,
    offending_text: Option<String>,
    explanation: String,
}

impl ParseError {
    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn offending_text(&self) -> Option<&str> {
        self.offending_text.as_deref()
    }

    pub fn explanation(&self) -> &str {
        &self.explanation
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {}",
            self.filename, self.line, self.explanation
        )?;
        if let Some(text) = &self.offending_text {
            write!(formatter, " (found `{text}`)")?;
        }
        Ok(())
    }
}

impl Error for ParseError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenKind<'a> {
    Word(&'a str),
    Quoted(&'a str),
    Period,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Token<'a> {
    kind: TokenKind<'a>,
    line: usize,
}

/// Parses the required first entry of an order file.
///
/// The accepted syntax is `game GAME-CODE turn TURN-NUMBER .`. Whitespace,
/// including newlines, separates tokens. The terminating period may be
/// attached to the turn number.
pub fn parse_order_file_header(
    filename: impl Into<String>,
    source: &str,
) -> Result<OrderFileHeader, ParseError> {
    let filename = filename.into();
    let tokens = tokenize(&filename, source)?;
    let eof_line = source.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let mut parser = Parser {
        filename: &filename,
        tokens: &tokens,
        position: 0,
        eof_line,
    };

    parser.parse_header()
}

/// Parses the required game/turn and authentication entries at the start of
/// an order file.
pub fn parse_order_file_preamble(
    filename: impl Into<String>,
    source: &str,
) -> Result<OrderFilePreamble, ParseError> {
    let filename = filename.into();
    let tokens = tokenize(&filename, source)?;
    let eof_line = source.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let mut parser = Parser {
        filename: &filename,
        tokens: &tokens,
        position: 0,
        eof_line,
    };

    let header = parser.parse_header()?;
    let (owner, token) = parser.parse_authentication()?;
    Ok(OrderFilePreamble {
        header,
        owner,
        token,
    })
}

/// Checks every order in a file and returns all syntax errors found.
///
/// Orders are terminated by periods. After an error, checking resumes at the
/// next order so that one invocation can report independent errors throughout
/// the file. The first two orders must be the game header and authentication
/// order, respectively.
pub fn check_order_file_syntax(filename: impl Into<String>, source: &str) -> Vec<ParseError> {
    let filename = filename.into();
    let eof_line = source.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let (tokens, mut errors) = tokenize_all(&filename, source);
    let mut start = 0;
    let mut order_index = 0;

    for end in (0..tokens.len()).filter(|index| tokens[*index].kind == TokenKind::Period) {
        check_order(
            &filename,
            &tokens[start..=end],
            eof_line,
            order_index,
            &mut errors,
        );
        order_index += 1;
        start = end + 1;
    }

    if start < tokens.len() {
        check_order(
            &filename,
            &tokens[start..],
            eof_line,
            order_index,
            &mut errors,
        );
        order_index += 1;
    }

    if order_index == 0 {
        errors.push(ParseError {
            filename: filename.clone(),
            line: eof_line,
            offending_text: None,
            explanation: "expected `game` as the first order".to_owned(),
        });
    }
    if order_index < 2 {
        errors.push(ParseError {
            filename,
            line: eof_line,
            offending_text: None,
            explanation: "expected `authenticate` as the second order".to_owned(),
        });
    }

    errors.sort_by_key(|error| error.line);
    errors
}

fn check_order(
    filename: &str,
    tokens: &[Token<'_>],
    eof_line: usize,
    order_index: usize,
    errors: &mut Vec<ParseError>,
) {
    let mut parser = Parser {
        filename,
        tokens,
        position: 0,
        eof_line,
    };
    let result = match order_index {
        0 => parser.parse_header().map(|_| ()),
        1 => parser.parse_authentication().map(|_| ()),
        _ => parser.parse_player_order(),
    };
    if let Err(error) = result.and_then(|()| parser.expect_end()) {
        errors.push(error);
    }
}

fn tokenize<'a>(filename: &str, source: &'a str) -> Result<Vec<Token<'a>>, ParseError> {
    let (tokens, errors) = tokenize_all(filename, source);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    Ok(tokens)
}

fn tokenize_all<'a>(filename: &str, source: &'a str) -> (Vec<Token<'a>>, Vec<ParseError>) {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut index = 0;
    let mut line = 1;

    while index < source.len() {
        let character = source[index..]
            .chars()
            .next()
            .expect("index is before the end of source");
        if character.is_whitespace() {
            if character == '\n' {
                line += 1;
            }
            index += character.len_utf8();
            continue;
        }

        if character == '"' {
            let quote_line = line;
            index += character.len_utf8();
            let content_start = index;
            let mut closing_quote = None;
            let mut malformed = false;

            while index < source.len() {
                let character = source[index..]
                    .chars()
                    .next()
                    .expect("index is before the end of source");
                if character == '"' {
                    closing_quote = Some(index);
                    break;
                }
                if character.is_control() || (character.is_whitespace() && character != ' ') {
                    if !malformed {
                        errors.push(ParseError {
                            filename: filename.to_owned(),
                            line,
                            offending_text: None,
                            explanation:
                                "quoted text may contain spaces, but not newlines or control characters"
                                    .to_owned(),
                        });
                    }
                    malformed = true;
                    if character == '\n' {
                        line += 1;
                        index += character.len_utf8();
                        break;
                    }
                }
                index += character.len_utf8();
            }

            let Some(closing_quote) = closing_quote else {
                if !malformed {
                    errors.push(ParseError {
                        filename: filename.to_owned(),
                        line: quote_line,
                        offending_text: None,
                        explanation: "unterminated quoted text".to_owned(),
                    });
                }
                continue;
            };
            if !malformed {
                tokens.push(Token {
                    kind: TokenKind::Quoted(&source[content_start..closing_quote]),
                    line: quote_line,
                });
            }
            index = closing_quote + '"'.len_utf8();

            if index < source.len() {
                let next = source[index..]
                    .chars()
                    .next()
                    .expect("index is before the end of source");
                if next == '.' {
                    tokens.push(Token {
                        kind: TokenKind::Period,
                        line,
                    });
                    index += next.len_utf8();
                } else if !next.is_whitespace() {
                    errors.push(ParseError {
                        filename: filename.to_owned(),
                        line,
                        offending_text: Some(next.to_string()),
                        explanation: "expected whitespace or `.` after quoted text".to_owned(),
                    });
                }
            }
            continue;
        }

        let word_start = index;
        while index < source.len() {
            let character = source[index..]
                .chars()
                .next()
                .expect("index is before the end of source");
            if character.is_whitespace() {
                break;
            }
            index += character.len_utf8();
        }
        push_word_or_terminated_word(&mut tokens, &source[word_start..index], line);
    }

    (tokens, errors)
}

fn push_word_or_terminated_word<'a>(tokens: &mut Vec<Token<'a>>, word: &'a str, line: usize) {
    if let Some(word) = word.strip_suffix('.') {
        if !word.is_empty() {
            tokens.push(Token {
                kind: TokenKind::Word(word),
                line,
            });
        }
        tokens.push(Token {
            kind: TokenKind::Period,
            line,
        });
    } else {
        tokens.push(Token {
            kind: TokenKind::Word(word),
            line,
        });
    }
}

struct Parser<'a, 'source> {
    filename: &'a str,
    tokens: &'a [Token<'source>],
    position: usize,
    eof_line: usize,
}

impl<'source> Parser<'_, 'source> {
    fn parse_header(&mut self) -> Result<OrderFileHeader, ParseError> {
        self.expect_word("game")?;
        let game = GameCode(self.take_word("game code")?.to_owned());
        self.expect_word("turn")?;
        let turn_text = self.take_word("turn number")?;
        let turn = turn_text.parse::<u32>().map_err(|_| {
            self.error_at_current_or_previous(
                Some(turn_text),
                "turn number must be an unsigned 32-bit integer",
            )
        })?;
        self.expect_period()?;

        Ok(OrderFileHeader {
            game,
            turn: Turn(turn),
        })
    }

    fn parse_authentication(
        &mut self,
    ) -> Result<(OrderFileOwner, AuthenticationToken), ParseError> {
        self.expect_word("authenticate")?;
        let owner_kind = self.take_word("`email`, `player`, or `faction`")?;
        let owner = match owner_kind {
            "email" => OrderFileOwner::Email(Email(self.take_word("email")?.to_owned())),
            "player" => {
                let text = self.take_word("player ID")?;
                OrderFileOwner::Player(PlayerId(self.parse_id(text, "player ID")?))
            }
            "faction" => {
                let text = self.take_word("faction ID")?;
                OrderFileOwner::Faction(FactionId(self.parse_id(text, "faction ID")?))
            }
            other => {
                return Err(self.error_at_current_or_previous(
                    Some(other),
                    "expected `email`, `player`, or `faction` after `authenticate`",
                ));
            }
        };
        self.expect_word("with")?;
        self.expect_word("token")?;
        let token = AuthenticationToken(self.take_quoted("authentication token")?.to_owned());
        self.expect_period()?;

        Ok((owner, token))
    }

    fn parse_player_order(&mut self) -> Result<(), ParseError> {
        let command = self.take_word("order name")?;
        match command {
            "MOVE" => {
                self.take_u64("entity ID")?;
                self.take_u64("destination ID")?;
            }
            "TRANSFER" => {
                self.take_u64("source entity ID")?;
                self.take_word("unit")?;
                let status = self.take_word("inventory status")?;
                if !matches!(status, "AVAILABLE" | "RESERVED" | "DAMAGED") {
                    return Err(self.error_at_current_or_previous(
                        Some(status),
                        "expected inventory status `AVAILABLE`, `RESERVED`, or `DAMAGED`",
                    ));
                }
                self.take_u64("quantity")?;
                self.take_u64("destination entity ID")?;
            }
            other => {
                return Err(self.error_at_current_or_previous(
                    Some(other),
                    "expected order `MOVE` or `TRANSFER`",
                ));
            }
        }
        self.expect_period()
    }

    fn take_u64(&mut self, description: &str) -> Result<u64, ParseError> {
        let text = self.take_word(description)?;
        self.parse_id(text, description)
    }

    fn parse_id(&self, text: &str, description: &str) -> Result<u64, ParseError> {
        text.parse::<u64>().map_err(|_| {
            self.error_at_current_or_previous(
                Some(text),
                format!("{description} must be an unsigned 64-bit integer"),
            )
        })
    }

    fn expect_word(&mut self, expected: &str) -> Result<(), ParseError> {
        let token = self
            .next_token()
            .ok_or_else(|| self.error(self.eof_line, None, format!("expected `{expected}`")))?;
        match token.kind {
            TokenKind::Word(word) if word == expected => Ok(()),
            TokenKind::Word(word) => {
                Err(self.error(token.line, Some(word), format!("expected `{expected}`")))
            }
            TokenKind::Quoted(_) => Err(self.error(
                token.line,
                Some("<quoted text>"),
                format!("expected `{expected}`"),
            )),
            TokenKind::Period => {
                Err(self.error(token.line, Some("."), format!("expected `{expected}`")))
            }
        }
    }

    fn take_word(&mut self, description: &str) -> Result<&'source str, ParseError> {
        let token = self
            .next_token()
            .ok_or_else(|| self.error(self.eof_line, None, format!("expected {description}")))?;
        match token.kind {
            TokenKind::Word(word) => Ok(word),
            TokenKind::Quoted(_) => Err(self.error(
                token.line,
                Some("<quoted text>"),
                format!("expected {description}"),
            )),
            TokenKind::Period => {
                Err(self.error(token.line, Some("."), format!("expected {description}")))
            }
        }
    }

    fn take_quoted(&mut self, description: &str) -> Result<&'source str, ParseError> {
        let token = self.next_token().ok_or_else(|| {
            self.error(
                self.eof_line,
                None,
                format!("expected quoted {description}"),
            )
        })?;
        match token.kind {
            TokenKind::Quoted(text) => Ok(text),
            TokenKind::Word(_) => {
                Err(self.error(token.line, None, format!("expected quoted {description}")))
            }
            TokenKind::Period => Err(self.error(
                token.line,
                Some("."),
                format!("expected quoted {description}"),
            )),
        }
    }

    fn expect_period(&mut self) -> Result<(), ParseError> {
        let token = self
            .next_token()
            .ok_or_else(|| self.error(self.eof_line, None, "expected `.`"))?;
        match token.kind {
            TokenKind::Period => Ok(()),
            TokenKind::Word(word) => Err(self.error(token.line, Some(word), "expected `.`")),
            TokenKind::Quoted(_) => {
                Err(self.error(token.line, Some("<quoted text>"), "expected `.`"))
            }
        }
    }

    fn expect_end(&mut self) -> Result<(), ParseError> {
        let Some(token) = self.next_token() else {
            return Ok(());
        };
        let text = match token.kind {
            TokenKind::Word(word) => word,
            TokenKind::Quoted(_) => "<quoted text>",
            TokenKind::Period => ".",
        };
        Err(self.error(token.line, Some(text), "unexpected text after end of order"))
    }

    fn next_token(&mut self) -> Option<Token<'source>> {
        let token = self.tokens.get(self.position).copied();
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    fn error_at_current_or_previous(
        &self,
        text: Option<&str>,
        explanation: impl Into<String>,
    ) -> ParseError {
        let line = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map_or(self.eof_line, |token| token.line);
        self.error(line, text, explanation)
    }

    fn error(
        &self,
        line: usize,
        offending_text: Option<&str>,
        explanation: impl Into<String>,
    ) -> ParseError {
        ParseError {
            filename: self.filename.to_owned(),
            line,
            offending_text: offending_text.map(str::to_owned),
            explanation: explanation.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_with_spaces() {
        let header = parse_order_file_header("orders.txt", "game ECRA-01 turn 7 .").unwrap();

        assert_eq!(header.game.as_str(), "ECRA-01");
        assert_eq!(header.turn.number(), 7);
    }

    #[test]
    fn treats_newlines_as_token_separators() {
        let source = include_str!("../tests/fixtures/orders/valid-header.orders");
        let header = parse_order_file_header("valid-header.orders", source).unwrap();

        assert_eq!(header.game.as_str(), "ECRA-01");
        assert_eq!(header.turn.number(), 7);
    }

    #[test]
    fn requires_header_to_be_the_first_entry() {
        let error =
            parse_order_file_header("orders.txt", "MOVE 1001 12.\ngame ECRA turn 7.").unwrap_err();

        assert_eq!(error.line(), 1);
        assert_eq!(error.offending_text(), Some("MOVE"));
        assert_eq!(error.explanation(), "expected `game`");
    }

    #[test]
    fn reports_the_line_of_an_invalid_turn_number() {
        let source = include_str!("../tests/fixtures/orders/invalid-header.orders");
        let error = parse_order_file_header("invalid-header.orders", source).unwrap_err();

        assert_eq!(error.filename(), "invalid-header.orders");
        assert_eq!(error.line(), 2);
        assert_eq!(error.offending_text(), Some("tomorrow"));
        assert_eq!(
            error.explanation(),
            "turn number must be an unsigned 32-bit integer"
        );
    }

    #[test]
    fn rejects_a_missing_period() {
        let error = parse_order_file_header("orders.txt", "game ECRA turn 7").unwrap_err();

        assert_eq!(error.line(), 1);
        assert_eq!(error.explanation(), "expected `.`");
    }

    #[test]
    fn malformed_input_never_panics() {
        for source in ["", ".", "game", "game .", "game ECRA", "game ECRA turn ."] {
            assert!(parse_order_file_header("bad.orders", source).is_err());
        }
    }

    #[test]
    fn parses_email_authentication_after_header() {
        let source = include_str!("../tests/fixtures/orders/valid-preamble.orders");
        let preamble = parse_order_file_preamble("valid-preamble.orders", source).unwrap();

        assert_eq!(preamble.header.game.as_str(), "ECRA-01");
        assert_eq!(preamble.header.turn.number(), 7);
        assert_eq!(
            preamble.owner,
            OrderFileOwner::Email(Email("admiral.sato@example.com".to_owned()))
        );
        assert_eq!(preamble.token.as_str(), "opaque token.value");
    }

    #[test]
    fn parses_player_and_faction_authentication() {
        let player = parse_order_file_preamble(
            "player.orders",
            "game ECRA turn 3. authenticate player 42 with token \"player secret\".",
        )
        .unwrap();
        let faction = parse_order_file_preamble(
            "faction.orders",
            "game ECRA turn 3.\nauthenticate faction 91 with token \"faction-secret\" .",
        )
        .unwrap();

        assert_eq!(player.owner, OrderFileOwner::Player(PlayerId(42)));
        assert_eq!(faction.owner, OrderFileOwner::Faction(FactionId(91)));
    }

    #[test]
    fn requires_authentication_immediately_after_header() {
        let error = parse_order_file_preamble(
            "orders.txt",
            "game ECRA turn 3. MOVE 1001 12. authenticate player 42 with token \"secret\".",
        )
        .unwrap_err();

        assert_eq!(error.offending_text(), Some("MOVE"));
        assert_eq!(error.explanation(), "expected `authenticate`");
    }

    #[test]
    fn reports_an_invalid_owner_id() {
        let source = include_str!("../tests/fixtures/orders/invalid-authentication.orders");
        let error = parse_order_file_preamble("invalid-authentication.orders", source).unwrap_err();

        assert_eq!(error.line(), 2);
        assert_eq!(error.offending_text(), Some("not-a-number"));
        assert_eq!(
            error.explanation(),
            "player ID must be an unsigned 64-bit integer"
        );
    }

    #[test]
    fn authentication_token_must_be_quoted() {
        let error = parse_order_file_preamble(
            "orders.txt",
            "game ECRA turn 3. authenticate player 42 with token not-quoted.",
        )
        .unwrap_err();

        assert_eq!(error.line(), 1);
        assert_eq!(error.offending_text(), None);
        assert_eq!(error.explanation(), "expected quoted authentication token");
    }

    #[test]
    fn quoted_text_rejects_newlines_and_control_characters() {
        for source in [
            "game ECRA turn 3. authenticate player 42 with token \"two\nlines\".",
            "game ECRA turn 3. authenticate player 42 with token \"tab\there\".",
            "game ECRA turn 3. authenticate player 42 with token \"delete\u{7f}here\".",
        ] {
            let error = parse_order_file_preamble("orders.txt", source).unwrap_err();
            assert_eq!(
                error.explanation(),
                "quoted text may contain spaces, but not newlines or control characters"
            );
        }
    }

    #[test]
    fn quoted_text_has_no_escape_or_embedded_quote_syntax() {
        let error = parse_order_file_preamble(
            "orders.txt",
            "game ECRA turn 3. authenticate player 42 with token \"first\"second\".",
        )
        .unwrap_err();

        assert_eq!(
            error.explanation(),
            "expected whitespace or `.` after quoted text"
        );
    }

    #[test]
    fn reports_unterminated_quoted_text() {
        let error = parse_order_file_preamble(
            "orders.txt",
            "game ECRA turn 3.\nauthenticate player 42 with token \"never closed",
        )
        .unwrap_err();

        assert_eq!(error.line(), 2);
        assert_eq!(error.explanation(), "unterminated quoted text");
    }

    #[test]
    fn syntax_check_reports_independent_errors_and_continues() {
        let errors = check_order_file_syntax(
            "bad.orders",
            concat!(
                "game ECRA turn tomorrow.\n",
                "authenticate player no with token \"secret\".\n",
                "MOVE entity 12.\n",
                "TRANSFER 1 FOOD LOST many 2.\n",
            ),
        );

        assert_eq!(errors.len(), 4);
        assert_eq!(errors[0].line(), 1);
        assert_eq!(errors[1].line(), 2);
        assert_eq!(errors[2].line(), 3);
        assert_eq!(errors[3].line(), 4);
    }

    #[test]
    fn syntax_check_requires_game_and_authentication_orders() {
        let empty_errors = check_order_file_syntax("empty.orders", "");
        assert_eq!(empty_errors.len(), 2);
        assert!(empty_errors[0].explanation().contains("first order"));
        assert!(empty_errors[1].explanation().contains("second order"));

        let errors =
            check_order_file_syntax("bad.orders", "MOVE 1 2.\nTRANSFER 1 FOOD AVAILABLE 2 3.");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].explanation(), "expected `game`");
        assert_eq!(errors[1].explanation(), "expected `authenticate`");
    }

    #[test]
    fn syntax_check_accepts_a_complete_order_file() {
        let errors = check_order_file_syntax(
            "valid.orders",
            concat!(
                "game ECRA turn 3.\n",
                "authenticate player 42 with token \"secret\".\n",
                "MOVE 1001 12.\n",
                "TRANSFER 1001 FOOD AVAILABLE 25 1002.\n",
            ),
        );

        assert!(errors.is_empty());
    }
}
