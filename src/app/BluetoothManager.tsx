import Adw from "gi://Adw"
import app from "ags/gtk4/app"
import {createState, For, onCleanup, With} from "ags"
import {Gtk} from "ags/gtk4"
import GObject from "gnim/gobject"
import {Shapes} from "../lib/ui/Shapes"
import AstalBluetooth from "gi://AstalBluetooth"
import {Log} from "../lib/Logger"
import {interval, Timer} from "ags/time"

export default function BluetoothManager(bluetooth: AstalBluetooth.Bluetooth) {
    const [currentAdapter, setCurrentAdapter] = createState(bluetooth.get_adapter())

    const [powerState, setPowerState] = createState(currentAdapter.get()?.powered ?? false)
    const [discoverableState, setDiscoverableState] = createState(currentAdapter.get()?.discoverable ?? false)
    const [discoverableTimeoutState, setDiscoverableTimeoutState] = createState(currentAdapter.get()?.discoverable_timeout ?? 0);
    const [adapterAliasState, setAliasState] = createState(currentAdapter.get()?.alias ?? "Unknown");
    const [devices, setDevices] = createState<AstalBluetooth.Device[]>([]);
    const [selectedDevice, setSelectedDevice] = createState<AstalBluetooth.Device | null>(null);

    const timeout_time_adjustment = new Gtk.Adjustment({
        step_increment: 1,
        lower: 0,
        upper: 3,
        value: discoverableTimeoutState.get(),
    });

    let win: Adw.Window;
    let breakpoint: Adw.Breakpoint;
    let splitView: Adw.OverlaySplitView;
    let showSidebarButton: Gtk.ToggleButton;
    // let toastOverlay: Adw.ToastOverlay;
    // let toolbarViewSidebar: Adw.ToolbarView;
    // let sidebarContentBox: Gtk.Box;
    let mainListBox: Gtk.ListBox;
    let mainStack: Gtk.Stack;
    // let deviceSettingsStackPage: Gtk.StackPage;
    // let bluetoothSettingsPage: Gtk.StackPage;

    let globalBluetoothTick: Timer | null = null

    function getDeviceIcon(device: AstalBluetooth.Device) {
        const icon = device.icon || "";
        if (icon.includes("audio") || icon.includes("headset") || icon.includes("headphone")) {
            return "audio-headphones-symbolic";
        } else if (icon.includes("phone") || icon.includes("modem")) {
            return "phone-symbolic";
        } else if (icon.includes("computer")) {
            return "computer-symbolic";
        } else if (icon.includes("input") || icon.includes("keyboard")) {
            return "input-keyboard-symbolic";
        } else if (icon.includes("mouse")) {
            return "input-mouse-symbolic";
        }
        return "bluetooth-symbolic";
    }

    function sortDevices(deviceList: AstalBluetooth.Device[]) {
        return [...deviceList].sort((a, b) => {
            const aPairedTrusted = a.paired && a.trusted;
            const bPairedTrusted = b.paired && b.trusted;

            if (aPairedTrusted && !bPairedTrusted) return -1;
            if (!aPairedTrusted && bPairedTrusted) return 1;

            if (aPairedTrusted && bPairedTrusted) {
                const aName = a.name || a.alias || "Unknown";
                const bName = b.name || b.alias || "Unknown";
                return aName.localeCompare(bName);
            }

            const aPairedNotTrusted = a.paired && !a.trusted;
            const bPairedNotTrusted = b.paired && !b.trusted;

            if (aPairedNotTrusted && !bPairedNotTrusted) return -1;
            if (!aPairedNotTrusted && bPairedNotTrusted) return 1;

            if (aPairedNotTrusted && bPairedNotTrusted) {
                const aName = a.name || a.alias || "Unknown";
                const bName = b.name || b.alias || "Unknown";
                return aName.localeCompare(bName);
            }

            const aRssi = a.rssi || -100;
            const bRssi = b.rssi || -100;

            if (aRssi !== bRssi) {
                return bRssi - aRssi; // Higher RSSI first (closer to 0 is better)
            }

            const aName = a.name || a.alias || "Unknown";
            const bName = b.name || b.alias || "Unknown";
            return aName.localeCompare(bName);
        });
    }

    onCleanup(() => {
        globalBluetoothTick?.cancel()
        win.destroy()
    })

    function applyCssForDeviceRow(position: number) {
        switch (position) {
            case 0:
                return `border-radius: ${Shapes.windowRadius}px ${Shapes.windowRadius}px 0 0;`
            case devices.get().length - 1:
                return `border-radius: 0 0 ${Shapes.windowRadius}px ${Shapes.windowRadius}px;`
            default:
                return ""
        }
    }

    return (
        <Adw.Window
            $={(self) => win = self}
            name="bluetoothctl.gui"
            application={app}
            widthRequest={475}
            heightRequest={575}
            onShow={() => {
                globalBluetoothTick = interval(1000, () => {
                    const adapter = currentAdapter.get();
                    setPowerState(adapter?.powered ?? false);

                    setDiscoverableState(adapter?.discoverable ?? false);

                    setDiscoverableTimeoutState(adapter?.discoverable_timeout ?? 0);
                    timeout_time_adjustment.set_value(adapter ? adapter.discoverable_timeout / 60 : 3);

                    setAliasState(adapter?.alias ?? "Unknown");

                    setDevices(sortDevices(bluetooth.get_devices()));
                })
            }}
            onCloseRequest={(self) => {
                Log.d("Bluetooth", "Window closed")
                self.hide()
                globalBluetoothTick?.cancel()
                return true
            }}
            title="Bluetooth manager"
        >
            <Adw.Breakpoint
                $={(self) => {
                    breakpoint = self;
                    self.condition = Adw.BreakpointCondition.new_length(
                        Adw.BreakpointConditionLengthType.MAX_WIDTH,
                        700,
                        Adw.LengthUnit.SP
                    );
                }}
            />

            <Adw.ToastOverlay
                // $={(self) => toastOverlay = self}
            >
                <Adw.OverlaySplitView
                    $={(self) => {
                        splitView = self

                        const collapsedGValue = new GObject.Value()
                        collapsedGValue.init(GObject.TYPE_BOOLEAN)
                        collapsedGValue.set_boolean(true)
                        breakpoint.add_setter(splitView, "collapsed", collapsedGValue);

                        const showSidebarGValue = new GObject.Value()
                        showSidebarGValue.init(GObject.TYPE_BOOLEAN)
                        showSidebarGValue.set_boolean(false)
                        breakpoint.add_setter(splitView, "show-sidebar", showSidebarGValue);

                        if (showSidebarButton) {
                            splitView.bind_property('show-sidebar', showSidebarButton, 'active', GObject.BindingFlags.BIDIRECTIONAL);
                        }
                    }}
                    pinSidebar={false}
                    enableHideGesture
                    enableShowGesture
                    halign={Gtk.Align.FILL}
                    valign={Gtk.Align.FILL}
                    sidebarPosition={Gtk.PackType.START}
                    sidebarWidthFraction={0.4}
                    sidebarWidthUnit={Adw.LengthUnit.SP}
                    minSidebarWidth={300}
                    maxSidebarWidth={350}
                    sidebar={
                        <Adw.ToolbarView
                            // $={(self) => toolbarViewSidebar = self}
                            topBarStyle={Adw.ToolbarStyle.FLAT}
                        >
                            <Adw.HeaderBar $type={"top"}></Adw.HeaderBar>
                            <Gtk.Box
                                // $={(self) => sidebarContentBox = self}
                                marginTop={12}
                                marginBottom={12}
                                marginStart={12}
                                marginEnd={12}
                            >
                                <Gtk.Box
                                    valign={Gtk.Align.FILL}
                                    orientation={Gtk.Orientation.VERTICAL}
                                    spacing={28}
                                >
                                    <Adw.PreferencesGroup
                                        title="Settings"
                                        description="General Bluetooth Settings"
                                    >
                                        <Gtk.ListBox
                                            $={(self) => {
                                                self.set_selection_mode(Gtk.SelectionMode.SINGLE);

                                                self.select_row(self.get_first_child() as Gtk.ListBoxRow);
                                                selectedDevice.subscribe(() => {
                                                    const device = selectedDevice.get();
                                                    if (!device) {
                                                        mainListBox.unselect_all()
                                                    } else {
                                                        self.unselect_all()
                                                    }
                                                });

                                            }}
                                            css={`
                                                border-radius: ${Shapes.windowRadius}px;
                                                background: var(--dialog-bg-color);
                                                color: var(--dialog-fg-color);
                                            `}
                                        >
                                            <Adw.ActionRow
                                                css={`
                                                    border-radius: ${Shapes.windowRadius}px;
                                                `}
                                                title="Bluetooth Settings"
                                                activatable
                                                onActivated={() => {
                                                    setSelectedDevice(null);
                                                    mainStack.set_visible_child_name("bluetooth_settings_page");
                                                }}
                                            />
                                        </Gtk.ListBox>
                                    </Adw.PreferencesGroup>

                                    <Adw.PreferencesGroup>
                                        <Gtk.ListBox
                                            css={`
                                                border-radius: ${Shapes.windowRadius}px;
                                                background: var(--dialog-bg-color);
                                                color: var(--dialog-fg-color);
                                            `}
                                        >
                                            <Adw.SwitchRow
                                                css={`
                                                    border-radius: ${Shapes.windowRadius}px;
                                                `}
                                                title="Scan"
                                                sensitive={currentAdapter.as(a => a != null)}
                                                active={currentAdapter.as(a => a?.discoverable ?? false)}
                                                onNotifyActive={(self) => {
                                                    if (self.active) {
                                                        currentAdapter.get()?.start_discovery()
                                                    } else {
                                                        currentAdapter.get()?.stop_discovery()
                                                    }
                                                }}
                                            />
                                        </Gtk.ListBox>
                                    </Adw.PreferencesGroup>

                                    <Adw.PreferencesGroup
                                        title="Devices"
                                        description="All the devices you've connected to"
                                    >

                                        <With value={devices.as(d => d.length === 0)}>
                                            {(isEmpty: boolean) => {
                                                if (isEmpty) {
                                                    return <Gtk.Box
                                                        heightRequest={250}
                                                        widthRequest={250}
                                                        orientation={Gtk.Orientation.VERTICAL}
                                                    >
                                                        <Gtk.Frame heightRequest={250}>
                                                            <Gtk.Box
                                                                valign={Gtk.Align.CENTER}
                                                                halign={Gtk.Align.CENTER}
                                                                orientation={Gtk.Orientation.VERTICAL}
                                                                spacing={20}
                                                            >
                                                                <Gtk.Image
                                                                    valign={Gtk.Align.CENTER}
                                                                    halign={Gtk.Align.CENTER}
                                                                    iconName="bluetooth-disconnected-symbolic"
                                                                    pixelSize={52}
                                                                    opacity={0.4}
                                                                />
                                                                <Gtk.Label
                                                                    label="No devices in range"
                                                                    opacity={0.4}
                                                                />
                                                            </Gtk.Box>
                                                        </Gtk.Frame>
                                                    </Gtk.Box>
                                                } else {
                                                    return <Gtk.ScrolledWindow
                                                        vexpand
                                                        propagateNaturalHeight
                                                        kineticScrolling
                                                        overlayScrolling
                                                    >
                                                        <Gtk.ListBox
                                                            $type={"start"}
                                                            $={(self) => {
                                                                mainListBox = self;
                                                                self.set_selection_mode(Gtk.SelectionMode.SINGLE);
                                                            }}
                                                            css={`
                                                                border-radius: ${Shapes.windowRadius}px;
                                                                background: var(--dialog-bg-color);
                                                                color: var(--dialog-fg-color);
                                                            `}
                                                            marginTop={12}
                                                            marginBottom={12}
                                                            valign={Gtk.Align.FILL}
                                                        >
                                                            <For
                                                                each={devices}
                                                                id={(device: AstalBluetooth.Device) =>
                                                                    `${device.name}|${device.alias}|${device.address}|${device.icon}|${device.connected}|${device.paired}|${device.trusted}`
                                                                }
                                                            >
                                                                {(device: AstalBluetooth.Device, index) => {
                                                                    Log.d("Bluetooth", `Device: ${device.name} ${device.trusted}`)
                                                                    return <Adw.ActionRow
                                                                        $={(self) => {
                                                                            if (selectedDevice.get()?.address === device.address) {
                                                                                mainListBox.select_row(self)
                                                                            }
                                                                        }}
                                                                        css={applyCssForDeviceRow(index.get())}
                                                                        title={device.name || device.alias || "Unknown Device"}
                                                                        subtitle={device.address}
                                                                        activatable
                                                                        onActivated={() => {
                                                                            setSelectedDevice(device);
                                                                            mainStack.set_visible_child_name("device_settings_page");
                                                                        }}
                                                                    >
                                                                        <Gtk.Box $type={"prefix"} spacing={8}>
                                                                            <Gtk.Image
                                                                                iconName={getDeviceIcon(device)}
                                                                                iconSize={Gtk.IconSize.LARGE}
                                                                            />
                                                                        </Gtk.Box>
                                                                        <Gtk.Box $type={"suffix"} spacing={8}>
                                                                            <Gtk.Image
                                                                                iconName={device.connected ? "network-wireless-signal-excellent-symbolic" : "network-wireless-offline-symbolic"}
                                                                                iconSize={Gtk.IconSize.NORMAL}
                                                                                tooltipText={device.connected ? "Connected" : "Disconnected"}
                                                                                opacity={device.connected ? 1.0 : 0.5}
                                                                            />
                                                                            <Gtk.Image
                                                                                iconName={device.paired ? "network-transmit-receive-symbolic" : "network-no-route-symbolic"}
                                                                                iconSize={Gtk.IconSize.NORMAL}
                                                                                tooltipText={device.paired ? "Paired" : "Not paired"}
                                                                                opacity={device.paired ? 1.0 : 0.5}
                                                                            />
                                                                            <Gtk.Image
                                                                                iconName={device.trusted ? "network-wireless-encrypted-symbolic" : "channel-insecure-symbolic"}
                                                                                iconSize={Gtk.IconSize.NORMAL}
                                                                                tooltipText={device.trusted ? "Trusted" : "Not trusted"}
                                                                                opacity={device.trusted ? 1.0 : 0.5}
                                                                            />
                                                                        </Gtk.Box>
                                                                    </Adw.ActionRow>
                                                                }}
                                                            </For>
                                                        </Gtk.ListBox>
                                                    </Gtk.ScrolledWindow>
                                                }
                                            }}
                                        </With>
                                    </Adw.PreferencesGroup>
                                </Gtk.Box>
                            </Gtk.Box>
                        </Adw.ToolbarView> as Gtk.Widget
                    }
                    content={
                        <Adw.ToolbarView>
                            <Adw.HeaderBar $type={"top"}>
                                <Gtk.Box>
                                    <Gtk.ToggleButton
                                        $type={"start"}
                                        $={(self) => {
                                            showSidebarButton = self

                                            const activeGValue = new GObject.Value()
                                            activeGValue.init(GObject.TYPE_BOOLEAN)
                                            activeGValue.set_boolean(false)
                                            breakpoint.add_setter(showSidebarButton, "active", activeGValue);
                                        }}
                                        iconName="sidebar-show-symbolic"
                                        active
                                        tooltipText="Hide Sidebar"
                                    />
                                </Gtk.Box>
                            </Adw.HeaderBar>

                            <Gtk.Stack
                                $={(self) => {
                                    mainStack = self;
                                    self.set_visible_child_name("bluetooth_settings_page");
                                }}
                                valign={Gtk.Align.START}
                                halign={Gtk.Align.FILL}
                                transitionType={Gtk.StackTransitionType.SLIDE_LEFT_RIGHT}
                            >
                                {/* --- Main Bluetooth Settings Page  --- */}
                                <Gtk.StackPage
                                    // $={(self) => bluetoothSettingsPage = self}
                                    name="bluetooth_settings_page"
                                    child={
                                        <Gtk.ScrolledWindow
                                            propagateNaturalHeight
                                            kineticScrolling
                                            overlayScrolling
                                            valign={Gtk.Align.START}
                                        >
                                            <Adw.Clamp
                                                orientation={Gtk.Orientation.HORIZONTAL}
                                                // unit={Adw.LengthUnit.SP}
                                                maximumSize={500}
                                                marginTop={32}
                                                marginBottom={32}
                                                marginStart={32}
                                                marginEnd={32}
                                            >
                                                <Gtk.Box
                                                    orientation={Gtk.Orientation.VERTICAL}
                                                    valign={Gtk.Align.START}
                                                    spacing={20}
                                                >
                                                    <Gtk.Box
                                                        valign={Gtk.Align.START}
                                                        halign={Gtk.Align.CENTER}
                                                        orientation={Gtk.Orientation.VERTICAL}
                                                        spacing={20}
                                                    >
                                                        <Gtk.Image iconName="bluetooth-symbolic" pixelSize={80}/>
                                                        <Gtk.Label
                                                            label="Bluetooth Settings"
                                                            useMarkup
                                                        />
                                                    </Gtk.Box>

                                                    <Adw.PreferencesGroup
                                                        title="Bluetooth Adapter Status"
                                                        description="What's this adapter doing?"
                                                    >
                                                        <Adw.SwitchRow
                                                            title="Powered"
                                                            active={powerState}
                                                            onNotifyActive={(self) => {
                                                                const adapter = currentAdapter.get();
                                                                Log.d("Bluetooth", `onNotifyActive - Powered: ${self.active} for ${adapter?.alias ?? "Unknown"}`)
                                                                if (adapter) {
                                                                    adapter.powered = self.active;
                                                                }
                                                            }}
                                                        />
                                                        <Adw.SwitchRow
                                                            title="Discoverable"
                                                            subtitle="visible to others?"
                                                            active={discoverableState}
                                                            onNotifyActive={(self) => {
                                                                const adapter = currentAdapter.get();
                                                                if (adapter) {
                                                                    adapter.discoverable = self.active;
                                                                }
                                                            }}
                                                        />
                                                    </Adw.PreferencesGroup>

                                                    <Adw.PreferencesGroup
                                                        title="Adapter Properties"
                                                        description="Information about the current bluetooth adapter."
                                                    >
                                                        <Adw.ExpanderRow
                                                            title="Current Bluetooth Adapter"
                                                            subtitle={currentAdapter.get()?.alias ?? "No adapter"}
                                                            showEnableSwitch={false}
                                                        >
                                                            {bluetooth.get_adapters().map((adapterItem) => (
                                                                <Adw.ActionRow
                                                                    title={adapterItem.alias}
                                                                    subtitle={adapterItem.address}
                                                                    activatable
                                                                    onActivated={() => {
                                                                        Log.d("Bluetooth", `Adapter selected: ${adapterItem.alias}`);
                                                                        setCurrentAdapter(adapterItem);
                                                                        setPowerState(adapterItem.powered);
                                                                        setDiscoverableState(adapterItem.discoverable);
                                                                        setDiscoverableTimeoutState(adapterItem.discoverable_timeout);
                                                                        setAliasState(adapterItem.alias);
                                                                    }}
                                                                >
                                                                    {currentAdapter.get()?.address === adapterItem.address && (
                                                                        <Gtk.Box $type={"suffix"}>
                                                                            <Gtk.Image iconName="object-select-symbolic"/>
                                                                        </Gtk.Box>
                                                                    )}
                                                                </Adw.ActionRow>
                                                            ))}
                                                        </Adw.ExpanderRow>

                                                        <Adw.SpinRow
                                                            title="Discoverable Timeout"
                                                            enableUndo
                                                            subtitle="in minutes"
                                                            climbRate={100}
                                                            wrap
                                                            adjustment={timeout_time_adjustment}
                                                            value={discoverableTimeoutState}
                                                            onNotifyValue={(self) => {
                                                                const adapter = currentAdapter.get();
                                                                Log.d("Bluetooth", `onNotifyValue - Discoverable Timeout: ${self.value} minutes for ${adapter?.alias ?? "Unknown"}`)
                                                                if (adapter) {
                                                                    adapter.discoverable_timeout = self.value * 60;
                                                                }
                                                            }}
                                                        />

                                                        <Adw.EntryRow
                                                            title="Adapter Name"
                                                            text={adapterAliasState}
                                                            inputPurpose={Gtk.InputPurpose.ALPHA}
                                                            showApplyButton
                                                            onApply={(self) => {
                                                                const adapter = currentAdapter.get();
                                                                if (adapter) {
                                                                    adapter.alias = self.text;
                                                                }
                                                            }}
                                                        />
                                                    </Adw.PreferencesGroup>

                                                    <Adw.PreferencesGroup
                                                        title="System Settings"
                                                        description="Manage how your system is set up. (not currently supported)"
                                                        sensitive={false}
                                                    >
                                                        <Adw.ActionRow
                                                            title="Auto Accept"
                                                            subtitle="Note: This feature requires additional BlueZ configuration"
                                                            sensitive={false}
                                                        >
                                                            <Gtk.Box $type={"suffix"}>
                                                                <Gtk.Switch
                                                                    valign={Gtk.Align.CENTER}
                                                                    sensitive={false}
                                                                />
                                                            </Gtk.Box>
                                                        </Adw.ActionRow>

                                                        <Adw.EntryRow
                                                            title="Received Files Location"
                                                            text="/home/$USER/Downloads/Bluetooth/"
                                                            showApplyButton
                                                            sensitive={false}
                                                        >
                                                            <Gtk.Box $type={"suffix"}>
                                                                <Gtk.Button
                                                                    iconName="folder-symbolic"
                                                                    marginBottom={8}
                                                                    marginTop={8}
                                                                    marginEnd={8}
                                                                    marginStart={8}
                                                                    tooltipText="Choose folder"
                                                                    sensitive={false}
                                                                />
                                                            </Gtk.Box>
                                                        </Adw.EntryRow>
                                                        <Adw.SwitchRow
                                                            title="Hide Unknown Devices"
                                                            subtitle="Stops Unknown Devices from showing up in device list"
                                                            sensitive={false}
                                                        />
                                                    </Adw.PreferencesGroup>
                                                </Gtk.Box>
                                            </Adw.Clamp>
                                        </Gtk.ScrolledWindow> as Gtk.Widget
                                    }
                                />

                                {/* --- Device Settings Page --- */}
                                <Gtk.StackPage
                                    // $={(self) => deviceSettingsStackPage = self}
                                    name="device_settings_page"
                                    child={
                                        <Gtk.ScrolledWindow
                                            propagateNaturalHeight
                                            kineticScrolling
                                            overlayScrolling
                                            valign={Gtk.Align.START}
                                        >
                                            <Adw.Clamp
                                                orientation={Gtk.Orientation.HORIZONTAL}
                                                // unit={Adw.LengthUnit.SP}
                                                maximumSize={500}
                                                marginTop={32}
                                                marginBottom={32}
                                                marginStart={32}
                                                marginEnd={32}
                                            >
                                                <Gtk.Box
                                                    orientation={Gtk.Orientation.VERTICAL}
                                                    valign={Gtk.Align.CENTER}
                                                    spacing={18}
                                                >
                                                    <Gtk.Box
                                                        valign={Gtk.Align.START}
                                                        halign={Gtk.Align.CENTER}
                                                        orientation={Gtk.Orientation.VERTICAL}
                                                        spacing={20}
                                                    >
                                                        <Gtk.Image
                                                            iconName={selectedDevice.as(d => d ? getDeviceIcon(d) : "bluetooth-symbolic")}
                                                            pixelSize={80}
                                                        />
                                                        <Gtk.Label
                                                            label={selectedDevice.as(d => `<span font_weight='bold' size='x-large'>${d ? (d.name || d.alias || "Unknown Device") : "Device Settings"}</span>`)}
                                                            useMarkup
                                                        />
                                                    </Gtk.Box>

                                                    <Adw.PreferencesGroup
                                                        title="Connection Properties"
                                                        description="Bluetooth connection information about this device."
                                                    >
                                                        <Adw.SwitchRow
                                                            title="Connected"
                                                            active={selectedDevice.as(d => d?.connected ?? false)}
                                                            onNotifyActive={(self) => {
                                                                const device = selectedDevice.get();
                                                                if (device) {
                                                                    self.sensitive = false
                                                                    if (self.active) {
                                                                        device.connect_device(
                                                                            (source_object, res, data) => {
                                                                                Log.i("Bluetooth", `connect_device: ${(res.get_source_object() as AstalBluetooth.Device)} for ${source_object?.name ?? source_object?.address ?? "Unknow"} (data: ${data})`);
                                                                                self.sensitive = true
                                                                            }
                                                                        )
                                                                    } else {
                                                                        device.disconnect_device(
                                                                            (source_object, res, data) => {
                                                                                Log.i("Bluetooth", `disconnect_device: ${(res.get_source_object() as AstalBluetooth.Device)} for ${source_object?.name ?? source_object?.address ?? "Unknow"} (data: ${data})`);
                                                                                self.sensitive = true
                                                                            }
                                                                        )
                                                                    }
                                                                }
                                                            }}
                                                        />

                                                        {/*<Adw.ExpanderRow*/}
                                                        {/*    title="Audio Profile"*/}
                                                        {/*    showEnableSwitch*/}
                                                        {/*    sensitive={false}*/}
                                                        {/*>*/}
                                                        {/*    /!* ActionRows pour les profils audio ici *!/*/}
                                                        {/*</Adw.ExpanderRow>*/}

                                                        {/*<Adw.ActionRow title="Send File To Device">*/}
                                                        {/*    <Gtk.Box $type={"suffix"} marginTop={6} marginBottom={6}>*/}
                                                        {/*        <Gtk.Button label="Choose File"/>*/}
                                                        {/*    </Gtk.Box>*/}
                                                        {/*</Adw.ActionRow>*/}
                                                    </Adw.PreferencesGroup>

                                                    <Adw.PreferencesGroup
                                                        title="Device Properties"
                                                        description="Information about this bluetooth device."
                                                    >
                                                        <Adw.EntryRow
                                                            title="Device Name"
                                                            text={selectedDevice.as(d => d?.alias || d?.name || "Unknown")}
                                                            inputPurpose={Gtk.InputPurpose.ALPHA}
                                                            showApplyButton
                                                            onApply={(self) => {
                                                                const device = selectedDevice.get();
                                                                if (device) {
                                                                    device.alias = self.text;
                                                                }
                                                            }}
                                                        />
                                                        <Adw.SwitchRow
                                                            title="Trusted"
                                                            active={selectedDevice.as(d => d?.trusted ?? false)}
                                                            onNotifyActive={(self) => {
                                                                const device = selectedDevice.get();
                                                                if (device) {
                                                                    device.trusted = self.active;
                                                                }
                                                            }}
                                                        />
                                                        <Adw.SwitchRow
                                                            title="Blocked"
                                                            active={selectedDevice.as(d => d?.blocked ?? false)}
                                                            onNotifyActive={(self) => {
                                                                const device = selectedDevice.get();
                                                                if (device) {
                                                                    device.blocked = self.active;
                                                                }
                                                            }}
                                                        />
                                                    </Adw.PreferencesGroup>

                                                    <Adw.PreferencesGroup
                                                        title="Status Information"
                                                        description="The current state of this device"
                                                    >
                                                        <Adw.ActionRow
                                                            title={"Name"}
                                                            subtitle={selectedDevice.as(d => d?.name || "######")}
                                                        />
                                                        <Adw.ActionRow
                                                            title={"Remote Device ID information"}
                                                            subtitle={selectedDevice.as(d => d?.modalias || "######")}
                                                        />
                                                        <Adw.ActionRow
                                                            title={"Alias"}
                                                            subtitle={selectedDevice.as(d => d?.alias || "######")}
                                                        />
                                                        <Adw.ActionRow
                                                            title={"Adapter"}
                                                            subtitle={selectedDevice.as(d => d?.adapter || "Unknow")}
                                                        />
                                                        <Adw.ActionRow
                                                            title={"Address"}
                                                            subtitle={selectedDevice.as(d => d?.address || "######")}
                                                        />


                                                        <With value={selectedDevice.as((d) => d?.battery_percentage ?? -1)}>
                                                            {(battery: number) => battery > -1 && (
                                                                <Adw.ActionRow
                                                                    title="Battery Level"
                                                                    subtitle={`${(battery * 100).toFixed(0)}%`}
                                                                >
                                                                    <Gtk.Box $type={"suffix"}>
                                                                        <Gtk.LevelBar
                                                                            value={battery}
                                                                            minValue={0}
                                                                            maxValue={1}
                                                                            widthRequest={100}
                                                                            valign={Gtk.Align.CENTER}
                                                                        />
                                                                    </Gtk.Box>
                                                                </Adw.ActionRow>

                                                            )}
                                                        </With>
                                                    </Adw.PreferencesGroup>

                                                    <Adw.PreferencesGroup>
                                                        <Gtk.Button
                                                            css={`
                                                                background: var(--error-bg-color);
                                                                color: var(--error-fg-color);
                                                            `}
                                                            label="Remove Device"
                                                            onClicked={() => {
                                                                const device = selectedDevice.get();
                                                                if (device) {
                                                                    const adapter = currentAdapter.get();
                                                                    if (adapter) {
                                                                        adapter.remove_device(device);
                                                                        setSelectedDevice(null);
                                                                        mainStack.set_visible_child_name("bluetooth_settings_page");
                                                                    }
                                                                }
                                                            }}
                                                        />
                                                    </Adw.PreferencesGroup>
                                                </Gtk.Box>
                                            </Adw.Clamp>
                                        </Gtk.ScrolledWindow> as Gtk.Widget
                                    }
                                />
                            </Gtk.Stack>
                        </Adw.ToolbarView> as Gtk.Widget
                    }
                />
            </Adw.ToastOverlay>
        </Adw.Window>
    )
}
