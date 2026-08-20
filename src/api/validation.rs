// routes/api/validation.rs
pub enum ValidationError {
    InvalidEmail,
    PassWordTooShort,
    OrganizationNameTaken
}
pub fn is_valid_email(s: &str) -> bool {todo!()}