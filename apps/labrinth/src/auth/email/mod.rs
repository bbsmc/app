use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Address, Message, SmtpTransport, Transport};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MailError {
    #[error("环境错误")]
    Env(#[from] dotenvy::Error),
    #[error("邮箱错误: {0}")]
    Mail(#[from] lettre::error::Error),
    #[error("地址解析错误: {0}")]
    Address(#[from] lettre::address::AddressError),
    #[error("SMTP 错误: {0}")]
    Smtp(#[from] lettre::transport::smtp::Error),
}

pub fn send_email_raw(
    to: String,
    subject: String,
    body: String,
) -> Result<(), MailError> {
    let email = Message::builder()
        .from(Mailbox::new(
            Some("BBSMC".to_string()),
            Address::new("bbsmc", "mc9y.net")?,
        ))
        .to(to.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(body)?;

    let username = dotenvy::var("SMTP_USERNAME")?;
    let password = dotenvy::var("SMTP_PASSWORD")?;
    let host = dotenvy::var("SMTP_HOST")?;
    let creds = Credentials::new(username, password);

    let mailer = SmtpTransport::relay(&host)?
        .port(465)
        .credentials(creds)
        .build();

    mailer.send(&email)?;

    Ok(())
}

pub fn send_email(
    to: String,
    email_title: &str,
    email_description: &str,
    line_two: &str,
    button_info: Option<(&str, &str)>,
) -> Result<(), MailError> {
    let mut email = if button_info.is_some() {
        include_str!("button_notif.html")
    } else {
        include_str!("auth_notif.html")
    }
    .replace("{{ email_title }}", email_title)
    .replace("{{ email_description }}", email_description)
    .replace("{{ line_one }}", email_description)
    .replace("{{ line_two }}", line_two);

    if let Some((button_title, button_link)) = button_info {
        email = email
            .replace("{{ button_title }}", button_title)
            .replace("{{ button_link }}", button_link);
    }

    send_email_raw(to, email_title.to_string(), email)?;

    Ok(())
}
