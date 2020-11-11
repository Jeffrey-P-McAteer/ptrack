
use app_dirs::AppInfo;

use std::env;

mod webserver;

// These constants may be read from anywhere in
// the program and change default behaviours
pub const HTTP_PORT: u64 = 8443;
pub const APP_NAME: &'static str = "PTrack";
pub const APP_INFO: AppInfo = AppInfo { name: "PTrack", author: "Jeffrey McAteer <jeffrey.p.mcateer@gmail.com>"};

fn main() {
  if let Some(arg) = env::args().skip(1).next() {
    let arg = &arg[..];
    if arg == "help" {
      println!("TODO not sure if we need arguments;");
      return;
    }
  }

  // Setup event handler for OS signals
  let e = ctrlc::set_handler(move || {
    std::process::exit(0);
  });
  if let Err(e) = e {
    println!("Error setting signal handler: {}", e);
  }

  // Run background threads in the foreground
  bg_main();
  
}

fn bg_main() {
  crossbeam::thread::scope(|s| {
    
    s.spawn(|_| {
      if let Err(e) = webserver::main() {
        println!("webserver error = {:?}", e);
        std::process::exit(1);
      }
    });

  }).expect("Error joining threads");
}


