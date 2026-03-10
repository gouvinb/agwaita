use libpulse_binding::{
    callbacks::ListResult,
    context::{
        self,
        Context,
        FlagSet,
        introspect::Introspector,
        subscribe::InterestMaskSet,
    },
    def::PortAvailable,
    mainloop::standard::{
        IterateResult,
        Mainloop,
    },
    operation::{
        self,
        Operation,
    },
    proplist::{
        Proplist,
        properties::APPLICATION_NAME,
    },
    volume::{
        ChannelVolumes,
        Volume,
    },
};
use log::{
    debug,
    error,
};
use operation::State;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        Mutex,
        mpsc,
        mpsc::Sender,
    },
    thread::{
        self,
        JoinHandle,
    },
    time::Duration,
};

const NORMAL: f64 = 65536.0; // libpulse_binding::volume::Volume::NORMAL.0

#[derive(Debug, Clone)]
pub struct AudioData {
    pub default_sink_name: Option<String>,
    pub sinks: Vec<Device>,
    pub current_volume: f64, // 0.0-1.5
    pub current_muted: bool,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub volume: ChannelVolumes,
    pub is_mute: bool,
}

pub struct AudioService {
    data: Arc<Mutex<AudioData>>,
    callbacks: Arc<Mutex<Vec<Box<dyn Fn(f64, bool) + Send>>>>,
    _listener_thread: Option<JoinHandle<()>>,
    commander_tx: Option<Sender<AudioCommand>>,
}

enum AudioCommand {
    SetVolume(
        /* sink_name */ String,
        /* new_channel_volumes */ ChannelVolumes,
    ),
    SetMute(/* sink_name */ String, /* is_muted*/ bool),
}

impl AudioService {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(AudioData {
            default_sink_name: None,
            sinks: Vec::new(),
            current_volume: 0.5,
            current_muted: false,
        }));

        let callbacks: Arc<Mutex<Vec<Box<dyn Fn(f64, bool) + Send>>>> = Arc::new(Mutex::new(Vec::new()));

        let (commander_tx, commander_rx) = mpsc::channel();

        let listener_thread = Self::start_listener_thread(data.clone(), callbacks.clone());

        Self::start_commander_thread(commander_rx);

        AudioService {
            data,
            callbacks,
            _listener_thread: Some(listener_thread),
            commander_tx: Some(commander_tx),
        }
    }

    fn start_listener_thread(data: Arc<Mutex<AudioData>>, callbacks: Arc<Mutex<Vec<Box<dyn Fn(f64, bool) + Send>>>>) -> JoinHandle<()> {
        thread::spawn(move || {
            match Self::create_pulse_context() {
                Ok((mut mainloop, mut context, introspector)) => {
                    context.subscribe(
                        InterestMaskSet::SERVER.union(InterestMaskSet::SINK),
                        |res| {
                            if !res {
                                error!("Audio subscription failed!");
                            }
                        },
                    );

                    // Initial fetch
                    Self::fetch_server_info(&mut mainloop, &introspector, data.clone());
                    Self::fetch_sinks(
                        &mut mainloop,
                        &introspector,
                        data.clone(),
                        callbacks.clone(),
                    );

                    let (event_tx, event_rx) = mpsc::channel();
                    context.set_subscribe_callback(Some(Box::new(move |facility, _operation, _idx| {
                        debug!("PulseAudio event: {:?}", facility);
                        let _ = event_tx.send(());
                    })));

                    // Polling loop to fetch sinks on events
                    loop {
                        if event_rx.try_recv().is_ok() {
                            Self::fetch_server_info(&mut mainloop, &introspector, data.clone());
                            Self::fetch_sinks(
                                &mut mainloop,
                                &introspector,
                                data.clone(),
                                callbacks.clone(),
                            );
                        }

                        match mainloop.iterate(false) {
                            IterateResult::Quit(_) | IterateResult::Err(_) => {
                                error!("PulseAudio mainloop error");
                                break;
                            },
                            IterateResult::Success(_) => {},
                        }

                        thread::sleep(Duration::from_millis(167));
                    }
                },
                Err(e) => {
                    error!("Failed to create PulseAudio context: {}", e);
                },
            }
        })
    }

    fn start_commander_thread(rx: mpsc::Receiver<AudioCommand>) {
        thread::spawn(move || match Self::create_pulse_context() {
            Ok((mut mainloop, _context, mut introspector)) => loop {
                while let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        AudioCommand::SetVolume(name, channel_volumes) => {
                            let op = introspector.set_sink_volume_by_name(&name, &channel_volumes, None);
                            Self::wait_for_operation(&mut mainloop, op);
                        },
                        AudioCommand::SetMute(name, mute) => {
                            let op = introspector.set_sink_mute_by_name(&name, mute, None);
                            Self::wait_for_operation(&mut mainloop, op);
                        },
                    }
                }

                match mainloop.iterate(false) {
                    IterateResult::Quit(_) | IterateResult::Err(_) => break,
                    IterateResult::Success(_) => {},
                }

                thread::sleep(Duration::from_millis(167));
            },
            Err(e) => {
                error!("Failed to create commander context: {}", e);
            },
        });
    }

    fn create_pulse_context() -> Result<(Mainloop, Context, Introspector), String> {
        let mut proplist = Proplist::new().ok_or("Failed to create proplist")?;
        proplist
            .set_str(APPLICATION_NAME, "agwaita")
            .map_err(|_| "Failed to set app name")?;

        let mut mainloop = Mainloop::new().ok_or("Failed to create mainloop")?;

        let mut context = Context::new_with_proplist(&mainloop, "agwaita", &proplist).ok_or("Failed to create context")?;

        context
            .connect(None, FlagSet::NOFLAGS, None)
            .map_err(|e| format!("Failed to connect: {:?}", e))?;

        // Poll until context is ready
        loop {
            match mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    return Err("Mainloop error during connection".to_string());
                },
                IterateResult::Success(_) => {
                    if context.get_state() == context::State::Ready {
                        break;
                    }
                },
            }
        }

        let introspector = context.introspect();

        Ok((mainloop, context, introspector))
    }

    fn wait_for_operation<T: ?Sized>(mainloop: &mut Mainloop, operation: Operation<T>) {
        // Poll until operation is done
        loop {
            match mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => break,
                IterateResult::Success(_) => {
                    if operation.get_state() == State::Done {
                        break;
                    }
                },
            }
        }
    }

    fn fetch_server_info(mainloop: &mut Mainloop, introspector: &Introspector, data: Arc<Mutex<AudioData>>) {
        let op = introspector.get_server_info(move |info| {
            let mut data = data.lock().unwrap();
            data.default_sink_name = info.default_sink_name.as_ref().map(|s| s.to_string());
            debug!("Default sink: {:?}", data.default_sink_name);
        });

        Self::wait_for_operation(mainloop, op);
    }

    fn fetch_sinks(
        mainloop: &mut Mainloop,
        introspector: &Introspector,
        data: Arc<Mutex<AudioData>>,
        callbacks: Arc<Mutex<Vec<Box<dyn Fn(f64, bool) + Send>>>>,
    ) {
        let sinks = Rc::new(RefCell::new(Vec::new()));

        let op = introspector.get_sink_info_list({
            let data = data.clone();
            let callbacks = callbacks.clone();
            let sinks = sinks.clone();

            move |result| {
                match result {
                    ListResult::Item(sink_info) => {
                        if sink_info
                            .ports
                            .iter()
                            .any(|p| p.available != PortAvailable::No)
                        {
                            sinks.borrow_mut().push(Device {
                                name: sink_info
                                    .name
                                    .as_ref()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                description: sink_info
                                    .description
                                    .as_ref()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                volume: sink_info.volume,
                                is_mute: sink_info.mute,
                            });
                        }
                    },
                    ListResult::End => {
                        let mut data = data.lock().unwrap();
                        data.sinks = sinks.borrow().clone();

                        if let Some(default_name) = &data.default_sink_name {
                            if let Some(sink) = data.sinks.iter().find(|s| &s.name == default_name) {
                                let volume = sink.volume.avg().0 as f64 / NORMAL;
                                let muted = sink.is_mute;

                                data.current_volume = volume;
                                data.current_muted = muted;

                                debug!("Updated: volume={:.2}, muted={}", volume, muted);

                                // Release lock before callbacks
                                drop(data);
                                let callbacks = callbacks.lock().unwrap();
                                for callback in callbacks.iter() {
                                    callback(volume, muted);
                                }
                            }
                        }
                    },
                    ListResult::Error => {
                        error!("Error fetching sink list");
                    },
                }
            }
        });

        Self::wait_for_operation(mainloop, op);
    }

    pub fn get_volume(&self) -> f64 {
        self.data.lock().unwrap().current_volume
    }

    pub fn is_muted(&self) -> bool {
        self.data.lock().unwrap().current_muted
    }

    pub fn set_volume(&self, volume: f64) {
        let volume = volume.clamp(0.0, 1.5);
        let data = self.data.lock().unwrap();

        if let Some(sink_name) = &data.default_sink_name {
            if let Some(sink) = data.sinks.iter().find(|s| &s.name == sink_name) {
                let mut channel_volumes = sink.volume;
                let vol_u32 = ((volume * NORMAL) as u32).min(0x10000 * 3 / 2);
                channel_volumes.scale(Volume(vol_u32));

                debug!(
                    "Set volume: {:.2} (channels: {})",
                    volume,
                    channel_volumes.len()
                );

                if let Some(tx) = &self.commander_tx {
                    let _ = tx.send(AudioCommand::SetVolume(sink_name.clone(), channel_volumes));
                }
            } else {
                debug!("Sink {} not found in sinks list", sink_name);
            }
        }
    }

    pub fn toggle_mute(&self) {
        let data = self.data.lock().unwrap();
        let new_muted = !data.current_muted;

        if let Some(sink_name) = &data.default_sink_name {
            if let Some(tx) = &self.commander_tx {
                let _ = tx.send(AudioCommand::SetMute(sink_name.clone(), new_muted));
                debug!("Toggle mute: {}", new_muted);
            }
        }
    }

    pub fn on_change<F>(&self, callback: F)
    where
        F: Fn(f64, bool) + Send + 'static,
    {
        self.callbacks.lock().unwrap().push(Box::new(callback));
    }

    /// Monitor audio changes - registers callback and sends initial state
    pub fn monitor_audio<F>(&self, callback: F) -> AudioMonitor
    where
        F: Fn(f64, bool) + Send + 'static,
    {
        // Send initial values
        let volume = self.get_volume();
        let muted = self.is_muted();
        callback(volume, muted);

        // Register for future changes
        self.on_change(callback);

        AudioMonitor
    }
}

/// Monitor for audio changes
pub struct AudioMonitor;
