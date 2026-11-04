use lettre::message::Mailbox;
use lettre::message::header::ContentType;
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
    let from_name =
        dotenvy::var("SMTP_FROM_NAME").unwrap_or_else(|_| "BBSMC".to_string());
    let from_user = dotenvy::var("SMTP_FROM_USER")
        .unwrap_or_else(|_| "noreply".to_string());
    let from_domain = dotenvy::var("SMTP_FROM_DOMAIN")?;

    let email = Message::builder()
        .from(Mailbox::new(
            Some(from_name),
            Address::new(&from_user, &from_domain)?,
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

/// 对插入 HTML 邮件模板的动态内容做转义，防止用户内容（如 issue 标题/正文）注入 HTML。
fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn send_email(
    to: String,
    email_title: &str,
    email_description: &str,
    line_two: &str,
    button_info: Option<(&str, &str)>,
) -> Result<(), MailError> {
    // 安全加固: 原先对模板做顺序 replace 且不转义，导致两个问题：
    // 1) 用户内容（issue 标题/正文等）作为原始 HTML 注入，可伪造官方邮件、插入钓鱼链接；
    // 2) 用户内容里出现 "{{ line_two }}" 之类占位符会在后续 replace 中被再次展开。
    // 改为先转义所有动态值，再用正则单次扫描替换占位符（替换文本不再被重新扫描）。
    let template = if button_info.is_some() {
        include_str!("button_notif.html")
    } else {
        include_str!("auth_notif.html")
    };

    let email_title_esc = html_escape(email_title);
    let email_description_esc = html_escape(email_description);
    let line_two_esc = html_escape(line_two);
    let button_title_esc = button_info.map(|(t, _)| html_escape(t));
    let button_link_esc = button_info.map(|(_, l)| html_escape(l));

    static PLACEHOLDER: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| {
            regex::Regex::new(r"\{\{ (\w+) \}\}").unwrap()
        });

    let email = PLACEHOLDER
        .replace_all(template, |caps: &regex::Captures| {
            match &caps[1] {
                "email_title" => email_title_esc.clone(),
                "email_description" | "line_one" => {
                    email_description_esc.clone()
                }
                "line_two" => line_two_esc.clone(),
                "button_title" => button_title_esc.clone().unwrap_or_default(),
                "button_link" => button_link_esc.clone().unwrap_or_default(),
                // 未知占位符原样保留
                _ => caps[0].to_string(),
            }
        })
        .into_owned();

    send_email_raw(to, email_title.to_string(), email)?;

    Ok(())
}
