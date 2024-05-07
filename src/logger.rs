use std::io::Write;
use std::task::Poll;

use serenity::builder::ExecuteWebhook;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::webhook::Webhook;
use serenity::Error;
use tokio::sync::mpsc::UnboundedSender;

use crate::constants::Constants;
use crate::get_rt;
use std::sync::Arc;
use serenity::all::WebhookId;

#[derive(Clone)]
pub struct Logger {
    token: &'static str,
    id: u64,
    context: Arc<Http>,
    channel: Webhook,
}

impl Logger {
    pub async fn say_str<T: Into<String>>(&self, wait: Option<bool>, message: T) -> Option<Message> {
        self.say(wait, ExecuteWebhook::new().content(message)).await
    }

    pub async fn say(&self, wait: Option<bool>, message: ExecuteWebhook) -> Option<Message>
    {
        match self
            .channel
            .execute(self.context.as_ref(), wait.unwrap_or(false), message)
            .await
        {
            Err(Error::Model(_)) => {
                tracing::error!("Token is reportedly none?");
                None
            }
            Err(Error::Http(_)) => {
                tracing::error!("Content is malformed, or the webhook's token is invalid.");
                None
            }
            Err(Error::Json(_)) => {
                tracing::error!("Received invalid json!");
                None
            }
            Err(_) => {
                tracing::error!(
                    "Error: Other. Reportedly, this should never happen, according to serenity?"
                );
                None
            }
            Ok(v) => v,
        }
    }

    pub fn new(webhook_id: u64, webhook_token: &'static str) -> Self {
        let context = Http::new(webhook_token);
        let channel = get_rt()
            .block_on((&context).get_webhook_with_token(WebhookId::from(webhook_id), webhook_token))
            .map_err(|e| tracing::error!("Getting webhook failed, due to: '{}'", e.to_string()))
            .expect("See Logs!");
        Logger {
            token: webhook_token,
            id: webhook_id,
            context: Arc::new(context),
            channel,
        }
    }
    fn new_default() -> Self {
        Self::new(
            Constants::get_constants().default_webhook_id,
            &Constants::get_constants().default_webhook_token,
        )
    }

    #[allow(static_mut_refs)]
    pub fn logger() -> &'static Logger {
        tracing::trace!("About to get Logger");
        //Use a OnceCell here, when the feature is stable?
        static mut L: Option<Logger> = None;
        unsafe { &mut L }.get_or_insert_with(|| {
            tracing::info!("Constructing new Logger");
            Logger::new_default()
        })
    }
}

#[derive(Clone)]
pub struct LoggerWriter<T> {
    tx: UnboundedSender<T>,
}

impl LoggerWriter<String> {
    //Don't allow creation, because UnboundedSender could be
    fn new() -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut open = true;
            let wrapper: fn(String) -> String = |v| format!("`{}`", v);
            while open {
                tokio::task::yield_now().await;
                match serenity::futures::future::poll_fn(|cx| match rx.poll_recv(cx) {
                    Poll::Ready(None) => {
                        open = false;
                        Poll::Ready(None)
                    }
                    Poll::Ready(Some(v)) => {
                        cx.waker().wake_by_ref();
                        Poll::Ready(Some(v))
                    }
                    Poll::Pending => Poll::Ready(None),
                })
                .await
                {
                    None => (),
                    Some(v) => {
                        Logger::logger().say_str(None, wrapper(v)).await;
                    }
                }
            }
        });
        LoggerWriter { tx }
    }

    #[allow(static_mut_refs)]
    pub fn logger_writer() -> &'static Self {
        tracing::trace!("About to get Logger");
        //Use a OnceCell here, when the feature is stable?
        static mut LW: Option<LoggerWriter<String>> = None;
        unsafe { &mut LW }.get_or_insert_with(|| {
            tracing::info!("Constructing new Logger");
            LoggerWriter::new()
        })
    }
}

impl<T: std::fmt::Display> LoggerWriter<T> {
    pub fn say_str_sync(&self, message: T) -> Result<(), std::io::Error> {
        if self.tx.is_closed() {
            tracing::error!("{}", &Constants::get_constants().webhook_closed);
            tracing::error!("Sender Stream is closed?, but object is still accessible?");
            tracing::error!(
                "The following message was attempted to be sent: '{}'",
                message
            );
            Err(std::io::Error::from(std::io::ErrorKind::NotConnected))
        } else {
            self.tx
                .send(message)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::Interrupted))
        }
    }
}

impl Write for &LoggerWriter<String> {
    ///Note: This will always return buf.len(), if successful, because of potential encoding issues,
    ///which would result in part of the buffer being retried, and sending a partial message.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let str = String::from_utf8_lossy(buf).to_string();
        if crate::is_logging_enabled("WEBHOOK_LOGGING_ENABLED".to_string()) {
            self.say_str_sync(str).map(|_| buf.len())
        } else {
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
