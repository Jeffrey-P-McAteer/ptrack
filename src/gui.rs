
use std::path::PathBuf;
use std::process::Command;
use std::str;

use webbrowser;

use crate::APP_NAME;
use crate::HTTP_PORT;


pub fn main() -> Result<(), Box<dyn std::error::Error>> {
  hide_console_on_windows();

  let app_local_url = format!("http://127.0.0.1:{}/", HTTP_PORT);

  webbrowser::open(&app_local_url[..])?;

  let mut app = systray::Application::new()?;

  app.add_menu_item(APP_NAME, move |_window| {
    if let Err(e) = webbrowser::open(&app_local_url[..]) {
      println!("e={}", e);
    }
    Ok::<_, systray::Error>(())
  })?;

  app.add_menu_item("Quit", |window| {
    window.quit();
    Ok::<_, systray::Error>(())
  })?;

  app.wait_for_message()?;

  Ok(())
}



// This fn does nothing on linux/unix machines
// and it calls winapi system calls to hide the console
// on windows.
// Users may set the environment variable NO_CONSOLE_DETATCH=1
// to prevent detatching from the console when the GUI is opened.
fn hide_console_on_windows() {
  #[cfg(target_os = "windows")]
  {
    use std::env;
    if let Ok(val) = env::var("NO_CONSOLE_DETATCH") {
      if val.contains("y") || val.contains("Y") || val.contains("1") {
        return;
      }
    }
    hide_console_on_windows_win();
  }
}

#[cfg(target_os = "windows")]
fn hide_console_on_windows_win() {
  //use std::ptr;
  //use winapi::um::wincon::GetConsoleWindow;
  //use winapi::um::winuser::{ShowWindow, SW_HIDE};

  // Below always hides console, even when run from cmd.exe
  // let window = unsafe {GetConsoleWindow()};
  // // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow
  // if window != ptr::null_mut() {
  //     unsafe {
  //         ShowWindow(window, SW_HIDE);
  //     }
  // }

  // Check if we are run from the console or just launched with explorer.exe
  let mut console_proc_list_buff: Vec<u32> = vec![0; 16];
  let num_procs = unsafe { winapi::um::wincon::GetConsoleProcessList(console_proc_list_buff.as_mut_ptr(), 16) };
  if num_procs == 1 {
    // We were launched from explorer.exe, detatch the console
    unsafe { winapi::um::wincon::FreeConsole() };
  }
  // Otherwise do nothing, we want console messages when run from the console.

}

