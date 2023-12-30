//Based on smoltcp's macro_rules
#[cfg(not(test))]
#[cfg(feature = "log")]
#[macro_export]
macro_rules! svc_log {
    (debug, $($arg:expr),*) => { log::debug!($($arg),*) };
    (info,  $($arg:expr),*) => { log::info!($($arg),*)  };
    (warn,  $($arg:expr),*) => { log::warn!($($arg),*)  };
}

#[cfg(test)]
#[cfg(feature = "log")]
#[macro_export]
macro_rules! svc_log {
    (debug, $($arg:expr),*) => { println!($($arg),*) };
    (info,  $($arg:expr),*) => { println!($($arg),*) };
    (warn,  $($arg:expr),*) => { println!($($arg),*) };
}

#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! svc_log {
    (debug, $($arg:expr),*) => { defmt::debug!($($arg),*) };
    (info,  $($arg:expr),*) => { defmt::info!($($arg),*)  };
    (warn,  $($arg:expr),*) => { defmt::warn!($($arg),*)  };
}

#[cfg(not(any(feature = "log", feature = "defmt")))]
#[macro_export]
macro_rules! svc_log {
    ($level:ident, $($arg:expr),*) => {{ $( let _ = $arg; )* }}
}

#[allow(unused)]
pub(crate) use svc_log;
