mod messages;

use crate::utils::thread::Threaded;
pub(crate) use messages::{Message, MessageData};

use log::error;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

//--------------------------------------Message Box-----------------------------------------------------
pub(crate) struct MessageBox {
  bus_sender: Sender<Message>,
  system_receiver: Receiver<Message>,
  should_close: bool,
}

impl MessageBox {
  pub(crate) fn check_messages(&mut self) -> Option<Message> {
    // We close down either when we receive the Stop message or the message channel closes for some reason
    // otherwise we return the message (or lack of)
    match self.system_receiver.try_recv() {
      Ok(message) => match message {
        Message::Stop => {
          self.should_close = true;
          None
        }
        _ => Some(message),
      },
      Err(TryRecvError::Empty) => None,
      Err(TryRecvError::Disconnected) => {
        self.should_close = true;
        None
      }
    }
  }

  pub(crate) fn post_message(&self, message: Message) {
    match self.bus_sender.send(message) {
      Ok(_) => (),
      Err(e) => error!("Failed to send a message to the bus: {}", e.to_string()),
    }
  }

  pub(crate) fn should_close(&self) -> bool {
    self.should_close
  }
}

//---------------------------------------------------Message Bus System-------------------------------------------------
pub(crate) struct MessageBus {
  bus_sender: Sender<Message>,
  bus_receiver: Receiver<Message>,
  system_senders: Vec<Sender<Message>>,
}

impl MessageBus {
  pub(crate) fn new() -> Self {
    let (bus_sender, bus_receiver) = std::sync::mpsc::channel();

    Self {
      bus_sender,
      bus_receiver,
      system_senders: Vec::new(),
    }
  }

  pub(crate) fn get_message_box(&mut self) -> MessageBox {
    let bus_sender = self.bus_sender.clone();
    let (system_sender, system_receiver) = std::sync::mpsc::channel();
    self.system_senders.push(system_sender);
    MessageBox {
      bus_sender,
      system_receiver,
      should_close: false,
    }
  }
}

impl Threaded for MessageBus {
  fn run(&mut self) {
    loop {
      let message = match self.bus_receiver.recv() {
        Ok(message) => message,
        Err(_) => {
          error! {"Message bus channel closed, cannot continue communication between systems!"};
          break;
        }
      };

      message.log_message();
      self.system_senders.iter().for_each(|sender| {
        match sender.send(message.clone()) {
          Ok(_) => (),
          Err(_) => error!("Failed to send a message to a system, channel already closed!"),
        };
      });

      if let Message::Stop = message {
        break;
      };
    }
  }

  fn name(&self) -> String {
    "Message Bus".to_owned()
  }
}
