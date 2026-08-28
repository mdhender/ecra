#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountRole {
    Administrator,
    User,
}

impl AccountRole {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Administrator => "administrator",
            Self::User => "user",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Account {
    number: u32,
    email: String,
    token: String,
    role: AccountRole,
}

impl Account {
    pub(crate) fn test_account(number: u32, role: AccountRole) -> Self {
        Self {
            number,
            email: format!("account.{number:04}@example.com"),
            token: format!("amp.rocks.{number:04}"),
            role,
        }
    }

    pub fn number(&self) -> u32 {
        self.number
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn role(&self) -> AccountRole {
        self.role
    }
}
