use std::io::{Write, ErrorKind, stdout};

use serenity::Error;
use serenity::builder::ExecuteWebhook;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::webhook::Webhook;

use crate::constants::Constants;
use tokio::sync::mpsc::{Sender, Receiver, UnboundedSender};
use std::time::Duration;
use std::thread::JoinHandle;
use std::cell::{Cell, RefCell};
use serenity::futures::{TryFuture, SinkExt};
use crate::{get_rt, main};
use tokio::runtime::Runtime;
use std::panic::take_hook;
use serenity::futures::task::SpawnExt;
use std::mem::MaybeUninit;
use tokio_util::sync::PollSender;
use std::task::Poll;
use std::sync::Arc;

type ChannelType<'a, 'b> = &'a mut ExecuteWebhook<'b>;

// trait Modify<'a,'b> = FnOnce(ChannelType) -> ChannelType;
pub struct Logger {
	token: &'static str,
	id: u64,
	context: Http,
	channel: Webhook,
}

impl Logger {
	// ///This method sends a message, and waits for the item to be send to a channel.
	// ///This means, that the item successfully entered a queue, to be sent to discord, not that it has started sending!
	// ///This is a blocking operation, due to the synchronous nature of this fn call.
	// pub fn say_str_sync(&self, message:String, block:Option<bool>) -> Result<(),std::io::Error>{
	// 	let while_conditional = block.unwrap_or(false);
	// 	tracing::trace!("Trying a sync send");
	// 	let mut out = Err(std::io::Error::from(std::io::ErrorKind::Other));
	// 	while while_conditional{
	// 		if self.tx.into_inner().is_closed() {
	// 			tracing::error!("Sender Stream is closed?, but object is still accessible?");
	// 			tracing::error!("The following message was attempted to be sent: '{}'", message);
	// 			out = Err(std::io::Error::from(std::io::ErrorKind::NotConnected));
	// 		} else if !self.tx.into_inner().is_ready() {
	// 			tracing::debug!("A Send operation is still in progress.");
	// 			if while_conditional {
	// 				tracing::debug!("Will try again later. Good luck!");
	// 			} else {
	// 				out = Err(std::io::Error::from(std::io::ErrorKind::WouldBlock));
	// 			}
	// 		} else {
	// 			out = self.tx.into_inner().start_send(message.clone()).map_err(|_|std::io::Error::from(std::io::ErrorKind::Interrupted));
	// 		}
	// 	};
	// 	out
	// }
	
	pub async fn say_str<T: ToString>(&self, wait: Option<bool>, message: T) -> Option<Message> {
		self.say(wait, |m| m.content(message)).await
	}
	
	pub async fn say<'b, F>(&self, wait: Option<bool>, message: F) -> Option<Message>
		where for<'c> F: FnOnce(ChannelType<'c, 'b>) -> ChannelType<'c, 'b> {
		match self.channel.execute(self.context.as_ref(), wait.unwrap_or(false), message).await {
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
				tracing::error!("Error: Other. Reportedly, this should never happen, according to serenity?");
				None
			}
			Ok(v) => v
		}
	}
	
	pub fn new(webhook_id: u64, webhook_token: &'static str) -> Self {
		
		let context = Http::new_with_token(webhook_token);
		let channel = logger_rt().block_on(
			(&context).get_webhook_with_token(webhook_id, webhook_token)
		).map_err(|_| tracing::error!("{}", &Constants::get_constants().webhook_err_str))
			.expect(&Constants::get_constants().webhook_err_str);
		Logger {
			token: webhook_token,
			id: webhook_id,
			context,
			channel,
		}
	}
	fn new_default() -> Self {
		Self::new(
			Constants::get_constants().default_webhook_id,
			&Constants::get_constants().default_webhook_token,
		)
	}
	
	pub fn logger() -> &'static Logger {
		tracing::trace!("About to get Logger");
		//Use a OnceCell here, when the feature is stable?
		static mut lw: Option<Logger> = None;
		unsafe { &mut lw }.get_or_insert_with(|| {
			tracing::info!("Constructing new Logger");
			Logger::new_default()
		}
		)
	}
}

pub fn logger_rt() -> &'static Runtime {
	tracing::trace!("About to get Logger Runtime");
	//Use a OnceCell here, when the feature is stable?
	static mut rt: Option<Runtime> = None;
	unsafe { &mut rt }.get_or_insert_with(|| {
		tracing::info!("Constructing new Logger Runtime");
		Runtime::new().expect("No Logger Runtime :-(")
	}
	)
}

const SHUTDOWN_WAIT:Duration = Duration::from_secs(1);

pub struct LoggerWriter<T>{
	tx:UnboundedSender<T>
}

impl LoggerWriter<String> {
	//Don't allow creation, because UnboundedSender could be
	fn new() -> Self{
		let (tx,mut rx) = tokio::sync::mpsc::unbounded_channel();
		tokio::spawn(async move{
			let mut  open = true;
			let wrapper:fn(String) -> String = |v| format!("`{}`",v);
			while open {
				tokio::task::yield_now().await;
				match serenity::futures::future::poll_fn(|cx| {
					match rx.poll_recv(cx) {
						Poll::Ready(None) =>{
							open=false;
							Poll::Ready(None)
						}
						Poll::Ready(Some(v)) => {
							cx.waker().wake_by_ref();
							Poll::Ready(Some(v))
						}
						Poll::Pending => {Poll::Ready(None)}
					}
				}).await {
					None => (),
					Some(v) => {
						Logger::logger().say_str(None,wrapper(v)).await;
					},
				}
			}
		});
		LoggerWriter {
			tx
		}
	}
	

	pub fn logger_writer() -> &'static Self {
		tracing::trace!("About to get Logger");
		//Use a OnceCell here, when the feature is stable?
		static mut lw: Option<LoggerWriter<String>> = None;
		unsafe { &mut lw }.get_or_insert_with(|| {
			tracing::info!("Constructing new Logger");
			LoggerWriter::new()
		}
		)
	}
	
}

impl <T:std::fmt::Display> LoggerWriter<T> {
	pub fn say_str_sync(&self,message:T) -> Result<(),std::io::Error>{
		if self.tx.is_closed() {
			tracing::error!("Sender Stream is closed?, but object is still accessible?");
			tracing::error!("The following message was attempted to be sent: '{}'", message);
			Err(std::io::Error::from(std::io::ErrorKind::NotConnected))
		}else {
			self.tx.send(message).map_err(|_|std::io::Error::from(std::io::ErrorKind::Interrupted))
		}
	}
}

impl Write for &LoggerWriter<String> {
	///Note: This will always return buf.len(), if successful, because of potential encoding issues,
	///which would result in part of the buffer being retried, and sending a partial message.
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let str = String::from_utf8_lossy(buf).to_string();
		self.say_str_sync(str).map(|_|buf.len())
		// match
		
		// self.say_str_sync(str).map(|_|buf.len())
		// {
		// 	Some(_) => Ok(buf.len()),
		// 	None => Err(std::io::Error::new(
		// 		std::io::ErrorKind::BrokenPipe,
		// 		"Printing too many Messages. Can't keep up. Messages have probably been dropped!")
		// 	),
		// }
		
	}
	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}


/* 	pub fn say_sync_str(&self, message: T) -> Option<()> {
		if self.tx.is_none() {
			tracing::warn!("{}", &Constants::get_constants().webhook_uninit);
			None
		} else if self.tx.as_ref().unwrap().is_closed() {
			tracing::warn!("{}", &Constants::get_constants().webhook_closed);
			None
		} else if self.tx.as_ref().unwrap().blocking_send(message).is_err() {
			tracing::warn!("{}", &Constants::get_constants().webhook_tx);
			None
		} else {
			tracing::debug!("Sent a message :-)");
			Some(())
		}
	}
	
	
	fn init(mut self) -> Self{
		
		self.channel = ;
		self.context = Some(context);
		let (tx, mut rx) = tokio::sync::mpsc::channel::<T>(CAPACITY);
		self.tx=Some(tx);
		(&self).process.set(Some(std::thread::spawn(|| {
			let mut open: bool = true;
			let rt = crate::get_rt();
			while open {
				match rt.block_on(serenity::futures::future::poll_fn(|cx|
					match (&mut rx).poll_recv(cx) {
						std::task::Poll::Pending => {
							std::task::Poll::Ready(None)
						}
						std::task::Poll::Ready(None) => {
							open = false;
							std::task::Poll::Ready(None)
						}
						std::task::Poll::Ready(Some(v)) => {
							std::task::Poll::Ready(Some(v))
						}
					}
				)) {
					None => {}
					Some(v) => {
						rt.block_on((&self).say_str(None, v));
					}
				}
				std::thread::yield_now();
			}
		})));
			self
	}
 */