use std::{mem::ManuallyDrop, thread::JoinHandle};

use log::{error, info};

pub(crate) trait Threaded {
  fn run(&mut self);
  fn name(&self) -> String;
}

pub(crate) struct Thread {
  name: String,
  thread: ManuallyDrop<JoinHandle<()>>,
}

impl Thread {
  pub(crate) fn new(mut thread: impl Threaded + Send + 'static) -> Self {
    let name = thread.name();
    info!("Creating thread: {}", name);
    let builder = std::thread::Builder::new().name(name.clone());
    let thread = builder
      .spawn(move || {
        thread.run();
      })
      .unwrap();

    Self {
      name,
      thread: ManuallyDrop::new(thread),
    }
  }

  pub(crate) fn is_finished(&self) -> bool {
    self.thread.is_finished()
  }
}

impl Drop for Thread {
  fn drop(&mut self) {
    info!("Joining on thread: {}", self.name);
    if let Err(err) = unsafe { ManuallyDrop::take(&mut self.thread) }.join() {
      let msg = match err.downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match err.downcast_ref::<String>() {
          Some(s) => &s[..],
          None => "unknown payload type",
        },
      };
      error!("Error while joining on {}:, {}", self.name, msg);
    }
  }
}
