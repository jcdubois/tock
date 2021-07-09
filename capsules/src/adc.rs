//! Syscall driver capsules for ADC sampling.
//!
//! This module has two ADC syscall driver capsule implementations.
//!
//! The first, called AdcDedicated, assumes that it has complete (dedicated)
//! control of the kernel ADC. This capsule provides userspace with
//! the ability to perform single, continuous, and high speed samples.
//! However, using this capsule means that no other
//! capsule or kernel service can use the ADC. It also allows only
//! a single process to use the ADC: other processes will receive
//! NOMEM errors.
//!
//! The second, called AdcVirtualized, sits top of an ADC virtualizer.
//! This capsule shares the ADC with the rest of the kernel through this
//! virtualizer, so allows other kernel services and capsules to use the
//! ADC. It also supports multiple processes requesting ADC samples
//! concurently. However, it only supports processes requesting single
//! ADC samples: they cannot sample continuously or at high speed.
//!
//!
//! Usage
//! -----
//!
//! ```rust
//! # use kernel::static_init;
//!
//! let adc_channels = static_init!(
//!     [&'static sam4l::adc::AdcChannel; 6],
//!     [
//!         &sam4l::adc::CHANNEL_AD0, // A0
//!         &sam4l::adc::CHANNEL_AD1, // A1
//!         &sam4l::adc::CHANNEL_AD3, // A2
//!         &sam4l::adc::CHANNEL_AD4, // A3
//!         &sam4l::adc::CHANNEL_AD5, // A4
//!         &sam4l::adc::CHANNEL_AD6, // A5
//!     ]
//! );
//! let adc = static_init!(
//!     capsules::adc::AdcDedicated<'static, sam4l::adc::Adc>,
//!     capsules::adc::AdcDedicated::new(
//!         &mut sam4l::adc::ADC0,
//!         adc_channels,
//!         &mut capsules::adc::ADC_BUFFER1,
//!         &mut capsules::adc::ADC_BUFFER2,
//!         &mut capsules::adc::ADC_BUFFER3
//!     )
//! );
//! sam4l::adc::ADC0.set_client(adc);
//! ```

use core::cell::Cell;
use core::convert::TryFrom;
use core::{cmp, mem};
use kernel::common::cells::{OptionalCell, TakeCell};
use kernel::hil;
use kernel::{
    CommandReturn, Driver, ErrorCode, Grant, ProcessId, ReadWriteProcessBuffer,
    ReadableProcessBuffer, WriteableProcessBuffer,
};

/// Syscall driver number.
use crate::driver;
use crate::virtual_adc::Operation;
pub const DRIVER_NUM: usize = driver::NUM::Adc as usize;

/// Multiplexed ADC syscall driver, used by applications and capsules.
/// Virtualized, and can be use by multiple applications at the same time;
/// requests are queued. Does not support continuous or high-speed sampling.
pub struct AdcVirtualized<'a> {
    drivers: &'a [&'a dyn hil::adc::AdcChannel],
    apps: Grant<AppSys, 1>,
    current_app: OptionalCell<ProcessId>,
}

/// ADC syscall driver, used by applications to interact with ADC.
/// Not currently virtualized: does not share the ADC with other capsules
/// and only one application can use it at a time. Supports continuous and
/// high speed sampling.
pub struct AdcDedicated<'a, A: hil::adc::Adc + hil::adc::AdcHighSpeed> {
    // ADC driver
    adc: &'a A,
    channels: &'a [&'a <A as hil::adc::Adc>::Channel],

    // ADC state
    active: Cell<bool>,
    mode: Cell<AdcMode>,

    // App state
    apps: Grant<App, 1>,
    appid: OptionalCell<ProcessId>,
    channel: Cell<usize>,

    // ADC buffers
    adc_buf1: TakeCell<'static, [u16]>,
    adc_buf2: TakeCell<'static, [u16]>,
    adc_buf3: TakeCell<'static, [u16]>,
}

/// ADC modes, used to track internal state and to signify to applications which
/// state a callback came from
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum AdcMode {
    NoMode = -1,
    SingleSample = 0,
    ContinuousSample = 1,
    SingleBuffer = 2,
    ContinuousBuffer = 3,
}

// Datas passed by the application to us
pub struct AppSys {
    pending_command: bool,
    command: OptionalCell<Operation>,
    channel: usize,
}

/// Holds buffers that the application has passed us
pub struct App {
    app_buf1: ReadWriteProcessBuffer,
    app_buf2: ReadWriteProcessBuffer,
    app_buf_offset: Cell<usize>,
    samples_remaining: Cell<usize>,
    samples_outstanding: Cell<usize>,
    next_samples_outstanding: Cell<usize>,
    using_app_buf1: Cell<bool>,
}

impl Default for App {
    fn default() -> App {
        App {
            app_buf1: ReadWriteProcessBuffer::default(),
            app_buf2: ReadWriteProcessBuffer::default(),
            app_buf_offset: Cell::new(0),
            samples_remaining: Cell::new(0),
            samples_outstanding: Cell::new(0),
            next_samples_outstanding: Cell::new(0),
            using_app_buf1: Cell::new(true),
        }
    }
}

impl Default for AppSys {
    fn default() -> AppSys {
        AppSys {
            pending_command: false,
            command: OptionalCell::empty(),
            channel: 0,
        }
    }
}
/// Buffers to use for DMA transfers
/// The size is chosen somewhat arbitrarily, but has been tested. At 175000 Hz,
/// buffers need to be swapped every 70 us and copied over before the next
/// swap. In testing, it seems to keep up fine.
pub static mut ADC_BUFFER1: [u16; 128] = [0; 128];
pub static mut ADC_BUFFER2: [u16; 128] = [0; 128];
pub static mut ADC_BUFFER3: [u16; 128] = [0; 128];

impl<'a, A: hil::adc::Adc + hil::adc::AdcHighSpeed> AdcDedicated<'a, A> {
    /// Create a new `Adc` application interface.
    ///
    /// - `adc` - ADC driver to provide application access to
    /// - `channels` - list of ADC channels usable by applications
    /// - `adc_buf1` - buffer used to hold ADC samples
    /// - `adc_buf2` - second buffer used when continuously sampling ADC
    pub fn new(
        adc: &'a A,
        grant: Grant<App, 1>,
        channels: &'a [&'a <A as hil::adc::Adc>::Channel],
        adc_buf1: &'static mut [u16; 128],
        adc_buf2: &'static mut [u16; 128],
        adc_buf3: &'static mut [u16; 128],
    ) -> AdcDedicated<'a, A> {
        AdcDedicated {
            // ADC driver
            adc: adc,
            channels: channels,

            // ADC state
            active: Cell::new(false),
            mode: Cell::new(AdcMode::NoMode),

            // App state
            apps: grant,
            appid: OptionalCell::empty(),
            channel: Cell::new(0),

            // ADC buffers
            adc_buf1: TakeCell::new(adc_buf1),
            adc_buf2: TakeCell::new(adc_buf2),
            adc_buf3: TakeCell::new(adc_buf3),
        }
    }

    /// Store a buffer we've regained ownership of and return a handle to it.
    /// The handle can have `map()` called on it in order to process the data in
    /// the buffer.
    ///
    /// - `buf` - buffer to be stored
    fn replace_buffer(&self, buf: &'static mut [u16]) -> &TakeCell<'static, [u16]> {
        // We play a little trick here where we always insert replaced buffers
        // in the last position but pull new buffers (in `take_and_map_buffer`)
        // from the beginning. We do this to get around Rust ownership rules
        // when handling errors. When we are doing continuous buffering, we need
        // to make sure that we re-gain ownership of the buffer passed back from
        // the ADC driver, AND we have to copy from that buffer the samples the
        // ADC driver took. To allow us to ensure we re-gain ownership, even if
        // an error occurs (like the app crashes), we unconditionally save
        // ownership of the returned buffer first (by calling this function).
        // However, we also pass zero or one buffers back to the ADC driver, and
        // we must ensure we do not pass the same buffer right back to the
        // driver before we have had a chance to save the samples.

        if self.adc_buf3.is_none() {
            self.adc_buf3.replace(buf);
        } else {
            let temp = self.adc_buf3.take();
            self.adc_buf3.replace(buf);

            // Find a place to insert the buffer we removed from the last slot.
            if self.adc_buf2.is_none() {
                temp.map(|likely_buffer| self.adc_buf2.replace(likely_buffer));
            } else {
                temp.map(|likely_buffer| self.adc_buf1.replace(likely_buffer));
            }
        }

        &self.adc_buf3
    }

    /// Find a buffer to give to the ADC to store samples in.
    ///
    /// - `closure` - function to run on the found buffer
    fn take_and_map_buffer<F: FnOnce(&'static mut [u16])>(&self, closure: F) {
        if self.adc_buf1.is_some() {
            self.adc_buf1.take().map(|val| {
                closure(val);
            });
        } else if self.adc_buf2.is_some() {
            self.adc_buf2.take().map(|val| {
                closure(val);
            });
        } else if self.adc_buf3.is_some() {
            self.adc_buf3.take().map(|val| {
                closure(val);
            });
        }
    }

    /// Collect a single analog sample on a channel.
    ///
    /// - `channel` - index into `channels` array, which channel to sample
    fn sample(&self, channel: usize) -> Result<(), ErrorCode> {
        // only one sample at a time
        if self.active.get() {
            return Err(ErrorCode::BUSY);
        }

        // convert channel index
        if channel >= self.channels.len() {
            return Err(ErrorCode::INVAL);
        }
        let chan = self.channels[channel];

        // save state for callback
        self.active.set(true);
        self.mode.set(AdcMode::SingleSample);
        self.channel.set(channel);

        // start a single sample
        let res = self.adc.sample(chan);
        if res != Ok(()) {
            // failure, clear state
            self.active.set(false);
            self.mode.set(AdcMode::NoMode);

            return res;
        }

        Ok(())
    }

    /// Collect repeated single analog samples on a channel.
    ///
    /// - `channel` - index into `channels` array, which channel to sample
    /// - `frequency` - number of samples per second to collect
    fn sample_continuous(&self, channel: usize, frequency: u32) -> Result<(), ErrorCode> {
        // only one sample at a time
        if self.active.get() {
            return Err(ErrorCode::BUSY);
        }

        // convert channel index
        if channel >= self.channels.len() {
            return Err(ErrorCode::INVAL);
        }
        let chan = self.channels[channel];

        // save state for callback
        self.active.set(true);
        self.mode.set(AdcMode::ContinuousSample);
        self.channel.set(channel);

        // start a single sample
        let res = self.adc.sample_continuous(chan, frequency);
        if res != Ok(()) {
            // failure, clear state
            self.active.set(false);
            self.mode.set(AdcMode::NoMode);

            return res;
        }

        Ok(())
    }

    /// Collect a buffer-full of analog samples.
    ///
    /// Samples are collected into the first app buffer provided. The number of
    /// samples collected is equal to the size of the buffer "allowed".
    ///
    /// - `channel` - index into `channels` array, which channel to sample
    /// - `frequency` - number of samples per second to collect
    fn sample_buffer(&self, channel: usize, frequency: u32) -> Result<(), ErrorCode> {
        // only one sample at a time
        if self.active.get() {
            return Err(ErrorCode::BUSY);
        }

        // convert channel index
        if channel >= self.channels.len() {
            return Err(ErrorCode::INVAL);
        }
        let chan = self.channels[channel];

        // cannot sample a buffer without a buffer to sample into
        let mut app_buf_length = 0;
        let exists = self.appid.map_or(false, |id| {
            self.apps
                .enter(*id, |state, _| {
                    app_buf_length = state.app_buf1.len();
                    app_buf_length > 0
                })
                .map_err(|err| {
                    if err == kernel::procs::Error::NoSuchApp
                        || err == kernel::procs::Error::InactiveApp
                    {
                        self.appid.clear();
                    }
                })
                .unwrap_or(false)
        });
        if !exists {
            return Err(ErrorCode::NOMEM);
        }

        // save state for callback
        self.active.set(true);
        self.mode.set(AdcMode::SingleBuffer);
        let ret = self.appid.map_or(Err(ErrorCode::NOMEM), |id| {
            self.apps
                .enter(*id, |app, _| {
                    app.app_buf_offset.set(0);
                    self.channel.set(channel);
                    // start a continuous sample
                    let res = self.adc_buf1.take().map_or(Err(ErrorCode::BUSY), |buf1| {
                        self.adc_buf2
                            .take()
                            .map_or(Err(ErrorCode::BUSY), move |buf2| {
                                // determine request length
                                let request_len = app_buf_length / 2;
                                let len1;
                                let len2;
                                if request_len <= buf1.len() {
                                    len1 = app_buf_length / 2;
                                    len2 = 0;
                                } else if request_len <= (buf1.len() + buf2.len()) {
                                    len1 = buf1.len();
                                    len2 = request_len - buf1.len();
                                } else {
                                    len1 = buf1.len();
                                    len2 = buf2.len();
                                }

                                // begin sampling
                                app.using_app_buf1.set(true);
                                app.samples_remaining.set(request_len - len1 - len2);
                                app.samples_outstanding.set(len1 + len2);
                                self.adc
                                    .sample_highspeed(chan, frequency, buf1, len1, buf2, len2)
                                    .map_or_else(
                                        |(ecode, buf1, buf2)| {
                                            // store buffers again
                                            self.replace_buffer(buf1);
                                            self.replace_buffer(buf2);
                                            Err(ecode)
                                        },
                                        |()| Ok(()),
                                    )
                            })
                    });
                    res
                })
                .map_err(|err| {
                    if err == kernel::procs::Error::NoSuchApp
                        || err == kernel::procs::Error::InactiveApp
                    {
                        self.appid.clear();
                    }
                })
                .unwrap_or(Err(ErrorCode::NOMEM))
        });
        if ret != Ok(()) {
            // failure, clear state
            self.active.set(false);
            self.mode.set(AdcMode::NoMode);
            self.appid.map(|id| {
                self.apps
                    .enter(*id, |app, _| {
                        app.samples_remaining.set(0);
                        app.samples_outstanding.set(0);
                    })
                    .map_err(|err| {
                        if err == kernel::procs::Error::NoSuchApp
                            || err == kernel::procs::Error::InactiveApp
                        {
                            self.appid.clear();
                        }
                    })
            });
        }
        ret
    }

    /// Collect analog samples continuously.
    ///
    /// Fills one "allowed" application buffer at a time and then swaps to
    /// filling the second buffer. Upcalls occur when the in use "allowed"
    /// buffer fills.
    ///
    /// - `channel` - index into `channels` array, which channel to sample
    /// - `frequency` - number of samples per second to collect
    fn sample_buffer_continuous(&self, channel: usize, frequency: u32) -> Result<(), ErrorCode> {
        // only one sample at a time
        if self.active.get() {
            return Err(ErrorCode::BUSY);
        }

        // convert channel index
        if channel >= self.channels.len() {
            return Err(ErrorCode::INVAL);
        }
        let chan = self.channels[channel];

        // cannot continuously sample without two buffers
        let mut app_buf_length = 0;
        let mut next_app_buf_length = 0;
        let exists = self.appid.map_or(false, |id| {
            self.apps
                .enter(*id, |state, _| {
                    app_buf_length = state.app_buf1.len();
                    next_app_buf_length = state.app_buf2.len();
                    state.app_buf1.len() > 0 && state.app_buf2.len() > 0
                })
                .map_err(|err| {
                    if err == kernel::procs::Error::NoSuchApp
                        || err == kernel::procs::Error::InactiveApp
                    {
                        self.appid.clear();
                    }
                })
                .unwrap_or(false)
        });
        if !exists {
            return Err(ErrorCode::NOMEM);
        }

        // save state for callback
        self.active.set(true);
        self.mode.set(AdcMode::ContinuousBuffer);

        let ret = self.appid.map_or(Err(ErrorCode::NOMEM), |id| {
            self.apps
                .enter(*id, |app, _| {
                    app.app_buf_offset.set(0);
                    self.channel.set(channel);
                    // start a continuous sample
                    self.adc_buf1.take().map_or(Err(ErrorCode::BUSY), |buf1| {
                        self.adc_buf2
                            .take()
                            .map_or(Err(ErrorCode::BUSY), move |buf2| {
                                // determine request lengths
                                let samples_needed = app_buf_length / 2;
                                let next_samples_needed = next_app_buf_length / 2;

                                // determine request lengths
                                let len1;
                                let len2;
                                if samples_needed <= buf1.len() {
                                    // we can fit the entire app_buffer request in the first
                                    // buffer. The second buffer will be used for the next
                                    // app_buffer
                                    len1 = samples_needed;
                                    len2 = cmp::min(next_samples_needed, buf2.len());
                                    app.samples_remaining.set(0);
                                    app.samples_outstanding.set(len1);
                                } else if samples_needed <= (buf1.len() + buf2.len()) {
                                    // we can fit the entire app_buffer request between the two
                                    // buffers
                                    len1 = buf1.len();
                                    len2 = samples_needed - buf1.len();
                                    app.samples_remaining.set(0);
                                    app.samples_outstanding.set(len1 + len2);
                                } else {
                                    // the app_buffer is larger than both buffers, so just
                                    // request max lengths
                                    len1 = buf1.len();
                                    len2 = buf2.len();
                                    app.samples_remaining.set(samples_needed - len1 - len2);
                                    app.samples_outstanding.set(len1 + len2);
                                }

                                // begin sampling
                                app.using_app_buf1.set(true);
                                self.adc
                                    .sample_highspeed(chan, frequency, buf1, len1, buf2, len2)
                                    .map_or_else(
                                        |(ecode, buf1, buf2)| {
                                            // store buffers again
                                            self.replace_buffer(buf1);
                                            self.replace_buffer(buf2);
                                            Err(ecode)
                                        },
                                        |()| Ok(()),
                                    )
                            })
                    })
                })
                .map_err(|err| {
                    if err == kernel::procs::Error::NoSuchApp
                        || err == kernel::procs::Error::InactiveApp
                    {
                        self.appid.clear();
                    }
                })
                .unwrap_or(Err(ErrorCode::NOMEM))
        });
        if ret != Ok(()) {
            // failure, clear state
            self.active.set(false);
            self.mode.set(AdcMode::NoMode);
            self.appid.map(|id| {
                self.apps
                    .enter(*id, |app, _| {
                        app.samples_remaining.set(0);
                        app.samples_outstanding.set(0);
                    })
                    .map_err(|err| {
                        if err == kernel::procs::Error::NoSuchApp
                            || err == kernel::procs::Error::InactiveApp
                        {
                            self.appid.clear();
                        }
                    })
            });
        }
        ret
    }

    /// Stops sampling the ADC.
    ///
    /// Any active operation by the ADC is canceled. No additional callbacks
    /// will occur. Also retrieves buffers from the ADC (if any).
    fn stop_sampling(&self) -> Result<(), ErrorCode> {
        if !self.active.get() || self.mode.get() == AdcMode::NoMode {
            // already inactive!
            return Ok(());
        }

        // clean up state
        self.appid.map_or(Err(ErrorCode::FAIL), |id| {
            self.apps
                .enter(*id, |app, _| {
                    self.active.set(false);
                    self.mode.set(AdcMode::NoMode);
                    app.app_buf_offset.set(0);

                    // actually cancel the operation
                    let rc = self.adc.stop_sampling();
                    if rc != Ok(()) {
                        return rc;
                    }

                    // reclaim buffers
                    match self.adc.retrieve_buffers() {
                        Ok((buf1, buf2)) => {
                            buf1.map(|buf| {
                                self.replace_buffer(buf);
                            });
                            buf2.map(|buf| {
                                self.replace_buffer(buf);
                            });
                            Ok(())
                        }
                        Err(ecode) => Err(ecode),
                    }
                })
                .map_err(|err| {
                    if err == kernel::procs::Error::NoSuchApp
                        || err == kernel::procs::Error::InactiveApp
                    {
                        self.appid.clear();
                    }
                })
                .unwrap_or(Err(ErrorCode::FAIL))
        })
    }

    fn get_resolution_bits(&self) -> usize {
        self.adc.get_resolution_bits()
    }

    fn get_voltage_reference_mv(&self) -> Option<usize> {
        self.adc.get_voltage_reference_mv()
    }
}

/// Functions to create, initialize, and interact with the virtualized ADC
impl<'a> AdcVirtualized<'a> {
    /// Create a new `Adc` application interface.
    ///
    /// - `drivers` - Virtual ADC drivers to provide application access to
    pub fn new(
        drivers: &'a [&'a dyn hil::adc::AdcChannel],
        grant: Grant<AppSys, 1>,
    ) -> AdcVirtualized<'a> {
        AdcVirtualized {
            drivers: drivers,
            apps: grant,
            current_app: OptionalCell::empty(),
        }
    }

    /// Enqueue the command to be executed when the ADC is available.
    fn enqueue_command(
        &self,
        command: Operation,
        channel: usize,
        appid: ProcessId,
    ) -> Result<(), ErrorCode> {
        if channel < self.drivers.len() {
            self.apps
                .enter(appid, |app, _| {
                    if self.current_app.is_none() {
                        self.current_app.set(appid);
                        let value = self.call_driver(command, channel);
                        if value != Ok(()) {
                            self.current_app.clear();
                        }
                        value
                    } else {
                        if app.pending_command == true {
                            Err(ErrorCode::BUSY)
                        } else {
                            app.pending_command = true;
                            app.command.set(command);
                            app.channel = channel;
                            Ok(())
                        }
                    }
                })
                .unwrap_or_else(|err| err.into())
        } else {
            Err(ErrorCode::NODEVICE)
        }
    }

    /// Request the sample from the specified channel
    fn call_driver(&self, command: Operation, channel: usize) -> Result<(), ErrorCode> {
        match command {
            Operation::OneSample => self.drivers[channel].sample(),
        }
    }
}

/// Callbacks from the ADC driver
impl<A: hil::adc::Adc + hil::adc::AdcHighSpeed> hil::adc::Client for AdcDedicated<'_, A> {
    /// Single sample operation complete.
    ///
    /// Collects the sample and provides a callback to the application.
    ///
    /// - `sample` - analog sample value
    fn sample_ready(&self, sample: u16) {
        let mut calledback = false;
        if self.active.get() && self.mode.get() == AdcMode::SingleSample {
            // single sample complete, clean up state
            self.active.set(false);
            self.mode.set(AdcMode::NoMode);

            // perform callback

            self.appid.map(|id| {
                self.apps
                    .enter(*id, |_app, upcalls| {
                        calledback = true;
                        upcalls.schedule_upcall(
                            0,
                            AdcMode::SingleSample as usize,
                            self.channel.get(),
                            sample as usize,
                        );
                    })
                    .map_err(|err| {
                        if err == kernel::procs::Error::NoSuchApp
                            || err == kernel::procs::Error::InactiveApp
                        {
                            self.appid.clear();
                        }
                    })
            });
        } else if self.active.get() && self.mode.get() == AdcMode::ContinuousSample {
            // sample ready in continuous sampling operation, keep state

            // perform callback
            self.appid.map(|id| {
                self.apps
                    .enter(*id, |_app, upcalls| {
                        calledback = true;
                        upcalls.schedule_upcall(
                            0,
                            AdcMode::ContinuousSample as usize,
                            self.channel.get(),
                            sample as usize,
                        );
                    })
                    .map_err(|err| {
                        if err == kernel::procs::Error::NoSuchApp
                            || err == kernel::procs::Error::InactiveApp
                        {
                            self.appid.clear();
                        }
                    })
            });
        }
        if !calledback {
            // operation probably canceled. Make sure state is consistent. No
            // callback
            self.active.set(false);
            self.mode.set(AdcMode::NoMode);

            // Also make sure that no more samples are taken if we were in
            // continuous mode.
            let _ = self.adc.stop_sampling();
        }
    }
}

/// Callbacks from the High Speed ADC driver
impl<A: hil::adc::Adc + hil::adc::AdcHighSpeed> hil::adc::HighSpeedClient for AdcDedicated<'_, A> {
    /// Internal buffer has filled from a buffered sampling operation.
    /// Copies data over to application buffer, determines if more data is
    /// needed, and performs a callback to the application if ready. If
    /// continuously sampling, also swaps application buffers and continues
    /// sampling when necessary. If only filling a single buffer, stops
    /// sampling operation when the application buffer is full.
    ///
    /// - `buf` - internal buffer filled with analog samples
    /// - `length` - number of valid samples in the buffer, guaranteed to be
    ///   less than or equal to buffer length
    fn samples_ready(&self, buf: &'static mut [u16], length: usize) {
        let mut unexpected_state = false;

        // Make sure in all cases we regain ownership of the buffer. However,
        // we also get a reference back to it so we can copy the sampled values
        // out and to an application.
        let buffer_with_samples = self.replace_buffer(buf);

        // do we expect a buffer?
        if self.active.get()
            && (self.mode.get() == AdcMode::SingleBuffer
                || self.mode.get() == AdcMode::ContinuousBuffer)
        {
            // we did expect a buffer. Determine the current application state
            self.appid.map(|id| {
                self.apps
                    .enter(*id, |app, upcalls| {
                        // determine which app buffer to copy data into and which is
                        // next up if we're in continuous mode
                        let use1 = app.using_app_buf1.get();
                        let next_app_buf;
                        let app_buf_ref;
                        if app.using_app_buf1.get() {
                            app_buf_ref = &app.app_buf1;
                            next_app_buf = &app.app_buf2;
                        } else {
                            app_buf_ref = &app.app_buf2;
                            next_app_buf = &app.app_buf1;
                        }

                        // update count of outstanding sample requests
                        app.samples_outstanding
                            .set(app.samples_outstanding.get() - length);

                        // provide a new buffer and length request to the ADC if
                        // necessary. If we haven't received enough samples for the
                        // current app_buffer, we may need to place more requests. If we
                        // have received enough, but are in continuous mode, we should
                        // place a request for the next app_buffer. This is all
                        // unfortunately made more complicated by the fact that there is
                        // always one outstanding request to the ADC.
                        let perform_callback;
                        if app.samples_remaining.get() == 0 {
                            // we have already placed outstanding requests for all the
                            // samples needed to fill the current app_buffer

                            if app.samples_outstanding.get() == 0 {
                                // and the samples we just received are the last ones
                                // we need
                                perform_callback = true;

                                if self.mode.get() == AdcMode::ContinuousBuffer {
                                    // it's time to switch to the next app_buffer, but
                                    // there's already an outstanding request to the ADC
                                    // for the next app_buffer that was placed last
                                    // time, so we need to account for that
                                    let samples_needed =
                                        next_app_buf.enter(|buf| buf.len() / 2).unwrap_or(0);
                                    app.samples_remaining
                                        .set(samples_needed - app.next_samples_outstanding.get());
                                    app.samples_outstanding
                                        .set(app.next_samples_outstanding.get());
                                    app.using_app_buf1.set(!app.using_app_buf1.get());

                                    // we also need to place our next request, however
                                    // the outstanding request already placed for the
                                    // next app_buffer might have completed it! So we
                                    // have to account for that case
                                    if app.samples_remaining.get() == 0 {
                                        // oh boy. We actually need to place a request
                                        // for the next next app_buffer (which is
                                        // actually the current app_buf, but try not to
                                        // think about that...). In practice, this
                                        // should be a pretty uncommon case to hit, only
                                        // occurring if the length of the app buffers
                                        // are smaller than the length of the adc
                                        // buffers, which is unsustainable at high
                                        // sampling frequencies
                                        let next_next_app_buf = &app_buf_ref;

                                        // provide a new buffer. However, we cannot
                                        // currently update state since the next
                                        // app_buffer still has a request outstanding.
                                        // We'll just make a request and handle the
                                        // state updating on next callback
                                        self.take_and_map_buffer(|adc_buf| {
                                            let samples_needed = next_next_app_buf
                                                .enter(|buf| buf.len() / 2)
                                                .unwrap_or(0);
                                            let request_len =
                                                cmp::min(samples_needed, adc_buf.len());
                                            app.next_samples_outstanding.set(request_len);
                                            let _ = self
                                                .adc
                                                .provide_buffer(adc_buf, request_len)
                                                .map_err(|(_, buf)| {
                                                    self.replace_buffer(buf);
                                                });
                                        });
                                    } else {
                                        // okay, we still need more samples for the next
                                        // app_buffer

                                        // provide a new buffer and update state
                                        self.take_and_map_buffer(|adc_buf| {
                                            let request_len = cmp::min(
                                                app.samples_remaining.get(),
                                                adc_buf.len(),
                                            );
                                            app.samples_remaining
                                                .set(app.samples_remaining.get() - request_len);
                                            app.samples_outstanding
                                                .set(app.samples_outstanding.get() + request_len);
                                            let _ = self
                                                .adc
                                                .provide_buffer(adc_buf, request_len)
                                                .map_err(|(_, buf)| {
                                                    self.replace_buffer(buf);
                                                });
                                        });
                                    }
                                }
                            } else {
                                // but there are still outstanding samples for the
                                // current app_buffer (actually exactly one request, the
                                // one the ADC is currently acting on)
                                perform_callback = false;

                                if self.mode.get() == AdcMode::ContinuousBuffer {
                                    // we're in continuous mode, so we need to start the
                                    // first request for the next app_buffer

                                    // provide a new buffer. However, we cannot
                                    // currently update state since the current
                                    // app_buffer still has a request outstanding. We'll
                                    // just make a request and handle the state updating
                                    // on next callback
                                    self.take_and_map_buffer(|adc_buf| {
                                        let samples_needed =
                                            next_app_buf.enter(|buf| buf.len() / 2).unwrap_or(0);
                                        let request_len = cmp::min(samples_needed, adc_buf.len());
                                        app.next_samples_outstanding.set(request_len);
                                        let _ = self
                                            .adc
                                            .provide_buffer(adc_buf, request_len)
                                            .map_err(|(_, buf)| {
                                                self.replace_buffer(buf);
                                            });
                                    });
                                }
                            }
                        } else {
                            // we need to get more samples for the current app_buffer
                            perform_callback = false;

                            // provide a new buffer and update state
                            self.take_and_map_buffer(|adc_buf| {
                                let request_len =
                                    cmp::min(app.samples_remaining.get(), adc_buf.len());
                                app.samples_remaining
                                    .set(app.samples_remaining.get() - request_len);
                                app.samples_outstanding
                                    .set(app.samples_outstanding.get() + request_len);
                                let _ = self.adc.provide_buffer(adc_buf, request_len).map_err(
                                    |(_, buf)| {
                                        self.replace_buffer(buf);
                                    },
                                );
                            });
                        }

                        let skip_amt = app.app_buf_offset.get() / 2;

                        {
                            let app_buf;
                            if use1 {
                                app_buf = &mut app.app_buf1;
                            } else {
                                app_buf = &mut app.app_buf2;
                            }

                            // next we should copy bytes to the app buffer
                            let _ = app_buf.mut_enter(|app_buf| {
                                // Copy bytes to app buffer by iterating over the
                                // data.
                                buffer_with_samples.map(|adc_buf| {
                                    // The `for` commands:
                                    //  * `chunks_mut`: get sets of two bytes from the app
                                    //                  buffer
                                    //  * `skip`: skips the already written bytes from the
                                    //            app buffer
                                    //  * `zip`: ties that iterator to an iterator on the
                                    //           adc buffer, limiting iteration length to
                                    //           the minimum of each of their lengths
                                    //  * `take`: limits us to the minimum of buffer lengths
                                    //            or sample length
                                    // We then split each sample into its two bytes and copy
                                    // them to the app buffer
                                    for (chunk, &sample) in app_buf
                                        .chunks(2)
                                        .skip(skip_amt)
                                        .zip(adc_buf.iter())
                                        .take(length)
                                    {
                                        let mut val = sample;
                                        for byte in chunk.iter() {
                                            byte.set((val & 0xFF) as u8);
                                            val = val >> 8;
                                        }
                                    }
                                });
                            });
                        }
                        // update our byte offset based on how many samples we
                        // copied
                        app.app_buf_offset
                            .set(app.app_buf_offset.get() + length * 2);

                        // let in_use_buf;
                        let (buf_ptr, buf_len) = if use1 {
                            (app.app_buf1.ptr(), app.app_buf1.len())
                        } else {
                            (app.app_buf2.ptr(), app.app_buf2.len())
                        };
                        // if the app_buffer is filled, perform callback
                        if perform_callback {
                            // actually schedule the callback
                            let len_chan = ((buf_len / 2) << 8) | (self.channel.get() & 0xFF);
                            upcalls.schedule_upcall(
                                0,
                                self.mode.get() as usize,
                                len_chan,
                                buf_ptr as usize,
                            );

                            // if the mode is SingleBuffer, the operation is
                            // complete. Clean up state
                            if self.mode.get() == AdcMode::SingleBuffer {
                                self.active.set(false);
                                self.mode.set(AdcMode::NoMode);
                                app.app_buf_offset.set(0);

                                // need to actually stop sampling
                                let _ = self.adc.stop_sampling();

                                // reclaim buffers and store them
                                if let Ok((buf1, buf2)) = self.adc.retrieve_buffers() {
                                    buf1.map(|buf| {
                                        self.replace_buffer(buf);
                                    });
                                    buf2.map(|buf| {
                                        self.replace_buffer(buf);
                                    });
                                }
                            } else {
                                // if the mode is ContinuousBuffer, we've just
                                // switched app buffers. Reset our offset to zero
                                app.app_buf_offset.set(0);
                            }
                        }
                    })
                    .map_err(|err| {
                        if err == kernel::procs::Error::NoSuchApp
                            || err == kernel::procs::Error::InactiveApp
                        {
                            self.appid.clear();
                            unexpected_state = true;
                        }
                    })
            });
        } else {
            unexpected_state = true;
        }

        if unexpected_state {
            // Operation was likely canceled, or the app crashed. Make sure
            // state is consistent. No callback.
            self.active.set(false);
            self.mode.set(AdcMode::NoMode);
            self.appid.map(|id| {
                self.apps
                    .enter(*id, |app, _| {
                        app.app_buf_offset.set(0);
                    })
                    .map_err(|err| {
                        if err == kernel::procs::Error::NoSuchApp
                            || err == kernel::procs::Error::InactiveApp
                        {
                            self.appid.clear();
                        }
                    })
            });

            // Make sure we do not take more samples since we know no app
            // is currently waiting on samples.
            let _ = self.adc.stop_sampling();

            // Also retrieve any buffers we passed to the underlying ADC driver.
            if let Ok((buf1, buf2)) = self.adc.retrieve_buffers() {
                buf1.map(|buf| {
                    self.replace_buffer(buf);
                });
                buf2.map(|buf| {
                    self.replace_buffer(buf);
                });
            }
        }
    }
}

/// Implementations of application syscalls
impl<A: hil::adc::Adc + hil::adc::AdcHighSpeed> Driver for AdcDedicated<'_, A> {
    /// Provides access to a buffer from the application to store data in or
    /// read data from.
    ///
    /// - `_appid` - application identifier, unused
    /// - `allow_num` - which allow call this is
    /// - `slice` - representation of application memory to copy data into
    fn allow_readwrite(
        &self,
        appid: ProcessId,
        allow_num: usize,
        mut slice: ReadWriteProcessBuffer,
    ) -> Result<ReadWriteProcessBuffer, (ReadWriteProcessBuffer, ErrorCode)> {
        // Return true if this app already owns the ADC capsule, if no app owns
        // the ADC capsule, or if the app that is marked as owning the ADC
        // capsule no longer exists.
        let match_or_empty_or_nonexistant = self.appid.map_or(true, |owning_app| {
            if self.active.get() {
                owning_app == &appid
            } else {
                self.apps
                    .enter(*owning_app, |_, _| owning_app == &appid)
                    .unwrap_or(true)
            }
        });
        if match_or_empty_or_nonexistant {
            self.appid.set(appid);
        } else {
            return Err((slice, ErrorCode::NOMEM));
        }
        match allow_num {
            // Pass buffer for samples to go into
            0 => {
                // set first buffer
                let res = self.appid.map_or(Err(ErrorCode::FAIL), |id| {
                    self.apps
                        .enter(*id, |app, _| {
                            mem::swap(&mut app.app_buf1, &mut slice);
                        })
                        .map_err(|err| {
                            if err == kernel::procs::Error::NoSuchApp
                                || err == kernel::procs::Error::InactiveApp
                            {
                                self.appid.clear();
                            }
                            ErrorCode::from(err)
                        })
                });
                if let Err(err) = res {
                    Err((slice, err))
                } else {
                    Ok(slice)
                }
            }

            // Pass a second buffer to be used for double-buffered continuous sampling
            1 => {
                // set second buffer
                let res = self.appid.map_or(Err(ErrorCode::FAIL), |id| {
                    self.apps
                        .enter(*id, |app, _| {
                            mem::swap(&mut app.app_buf2, &mut slice);
                        })
                        .map_err(|err| {
                            if err == kernel::procs::Error::NoSuchApp
                                || err == kernel::procs::Error::InactiveApp
                            {
                                self.appid.clear();
                            }
                            ErrorCode::from(err)
                        })
                });
                if let Err(err) = res {
                    Err((slice, err))
                } else {
                    Ok(slice)
                }
            }

            // default
            _ => Err((slice, ErrorCode::NOSUPPORT)),
        }
    }

    /// Method for the application to command or query this driver.
    ///
    /// - `command_num` - which command call this is
    /// - `data` - value sent by the application, varying uses
    /// - `_appid` - application identifier, unused
    fn command(
        &self,
        command_num: usize,
        channel: usize,
        frequency: usize,
        appid: ProcessId,
    ) -> CommandReturn {
        // Return true if this app already owns the ADC capsule, if no app owns
        // the ADC capsule, or if the app that is marked as owning the ADC
        // capsule no longer exists.
        let match_or_empty_or_nonexistant = self.appid.map_or(true, |owning_app| {
            // We have recorded that an app has ownership of the ADC.

            // If the ADC is still active, then we need to wait for the operation
            // to finish and the app, whether it exists or not (it may have crashed),
            // still owns this capsule. If the ADC is not active, then
            // we need to verify that that application still exists, and remove
            // it as owner if not.
            if self.active.get() {
                owning_app == &appid
            } else {
                // Check the app still exists.
                //
                // If the `.enter()` succeeds, then the app is still valid, and
                // we can check if the owning app matches the one that called
                // the command. If the `.enter()` fails, then the owning app no
                // longer exists and we return `true` to signify the
                // "or_nonexistant" case.
                self.apps
                    .enter(*owning_app, |_, _| owning_app == &appid)
                    .unwrap_or(true)
            }
        });
        if match_or_empty_or_nonexistant {
            self.appid.set(appid);
        } else {
            return CommandReturn::failure(ErrorCode::NOMEM);
        }
        match command_num {
            // check if present
            0 => CommandReturn::success_u32(self.channels.len() as u32),

            // Single sample on channel
            1 => match self.sample(channel) {
                Ok(()) => CommandReturn::success(),
                e => CommandReturn::failure(if let Ok(err) = ErrorCode::try_from(e) {
                    err
                } else {
                    panic!("ADC: invalid return code")
                }),
            },

            // Repeated single samples on a channel
            2 => match self.sample_continuous(channel, frequency as u32) {
                Ok(()) => CommandReturn::success(),
                e => CommandReturn::failure(if let Ok(err) = ErrorCode::try_from(e) {
                    err
                } else {
                    panic!("ADC: invalid return code")
                }),
            },

            // Multiple sample on a channel
            3 => match self.sample_buffer(channel, frequency as u32) {
                Ok(()) => CommandReturn::success(),
                e => CommandReturn::failure(if let Ok(err) = ErrorCode::try_from(e) {
                    err
                } else {
                    panic!("ADC: invalid return code")
                }),
            },

            // Continuous buffered sampling on a channel
            4 => match self.sample_buffer_continuous(channel, frequency as u32) {
                Ok(()) => CommandReturn::success(),
                e => CommandReturn::failure(if let Ok(err) = ErrorCode::try_from(e) {
                    err
                } else {
                    panic!("ADC: invalid return code")
                }),
            },

            // Stop sampling
            5 => match self.stop_sampling() {
                Ok(()) => CommandReturn::success(),
                e => CommandReturn::failure(if let Ok(err) = ErrorCode::try_from(e) {
                    err
                } else {
                    panic!("ADC: invalid return code")
                }),
            },

            // Get resolution bits
            101 => CommandReturn::success_u32(self.get_resolution_bits() as u32),
            // Get voltage reference mV
            102 => {
                if let Some(voltage) = self.get_voltage_reference_mv() {
                    CommandReturn::success_u32(voltage as u32)
                } else {
                    CommandReturn::failure(ErrorCode::NOSUPPORT)
                }
            }

            // default
            _ => CommandReturn::failure(ErrorCode::NOSUPPORT),
        }
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::procs::Error> {
        self.apps.enter(processid, |_, _| {})
    }
}

/// Implementation of the syscalls for the virtualized ADC.
impl Driver for AdcVirtualized<'_> {
    /// Method for the application to command or query this driver.
    ///
    /// - `command_num` - which command call this is
    /// - `channel` - requested channel value
    /// - `_` - value sent by the application, unused
    /// - `appid` - application identifier
    fn command(
        &self,
        command_num: usize,
        channel: usize,
        _: usize,
        appid: ProcessId,
    ) -> CommandReturn {
        match command_num {
            // This driver exists and return the number of channels
            0 => CommandReturn::success_u32(self.drivers.len() as u32),

            // Single sample.
            1 => {
                let res = self.enqueue_command(Operation::OneSample, channel, appid);
                if res == Ok(()) {
                    CommandReturn::success()
                } else {
                    match ErrorCode::try_from(res) {
                        Ok(error) => CommandReturn::failure(error),
                        _ => panic!("ADC Syscall: invalid errior from enqueue_command"),
                    }
                }
            }

            // Get resolution bits
            101 => {
                if channel < self.drivers.len() {
                    CommandReturn::success_u32(self.drivers[channel].get_resolution_bits() as u32)
                } else {
                    CommandReturn::failure(ErrorCode::NODEVICE)
                }
            }

            // Get voltage reference mV
            102 => {
                if channel < self.drivers.len() {
                    if let Some(voltage) = self.drivers[channel].get_voltage_reference_mv() {
                        CommandReturn::success_u32(voltage as u32)
                    } else {
                        CommandReturn::failure(ErrorCode::NOSUPPORT)
                    }
                } else {
                    CommandReturn::failure(ErrorCode::NODEVICE)
                }
            }

            _ => CommandReturn::failure(ErrorCode::NOSUPPORT),
        }
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::procs::Error> {
        self.apps.enter(processid, |_, _| {})
    }
}

impl<'a> hil::adc::Client for AdcVirtualized<'a> {
    fn sample_ready(&self, sample: u16) {
        self.current_app.take().map(|appid| {
            let _ = self.apps.enter(appid, |app, upcalls| {
                app.pending_command = false;
                let channel = app.channel;
                upcalls.schedule_upcall(
                    0,
                    AdcMode::SingleSample as usize,
                    channel,
                    sample as usize,
                );
            });
        });
    }
}
