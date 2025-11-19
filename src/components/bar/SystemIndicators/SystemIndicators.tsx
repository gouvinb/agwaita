import {Gtk} from "ags/gtk4"
import Adw from "gi://Adw"
import BrightnessIcon from "./icons/Brightness"
import AudioIcon from "./icons/Audio"
import BluetoothIcon from "./icons/Bluetooth"
import DoNotDisturbIcon from "./icons/DoNotDisturb"
import NetworkIcon from "./icons/Network"
import PowerModeIcon from "./icons/PowerMode";
import BatteryIcon from "./icons/Battery";
import AvatarIcon from "./icons/Avatar";
import DesktopScriptsLib, {shAsync} from "../../../lib/ExternalCommand";
import BrightnessQS from "./quick-settings/Brightness";
import AudioQS from "./quick-settings/Audio";
import PowerModeButtonQS from "./quick-settings/PowerMode";
import PowerModeRevealerQS from "./quick-settings/revealer/PowerMode";
import DarkModeButtonQS from "./quick-settings/DarkMode";
import BluetoothButtonQS from "./quick-settings/Bluetooth";
import {AirplaneModeButtonQS} from "./quick-settings/AirplaneMode";
import DoNotDisturbButtonQS from "./quick-settings/DoNotDisturb";
import AccentColorButtonQS from "./quick-settings/AccentColor";
import AccentColorRevealerQS from "./quick-settings/revealer/AccentColor";
import {Dimensions} from "../../../lib/ui/Dimensions";
import {createLifecycle} from "../../../lib/Lifecyle";
import {onCleanup} from "ags";

import AstalBattery from "gi://AstalBattery"
import AstalBluetooth from "gi://AstalBluetooth"
import AstalNotifd from "gi://AstalNotifd"
import AstalWp from "gi://AstalWp"
import PowerProfiles from "gi://AstalPowerProfiles"
import Brightness from "../../../services/Brightness";

interface SystemIndicatorsProps {
    notifd: AstalNotifd.Notifd,
    bluetooth: AstalBluetooth.Bluetooth,
    wp: AstalWp.Wp,
    battery: AstalBattery.Device,
    powerprofiles: PowerProfiles.PowerProfiles,
    brightness: Brightness,
}

export function SystemIndicators(
    {
        notifd,
        bluetooth,
        wp,
        battery,
        powerprofiles,
        brightness
    }: SystemIndicatorsProps
) {
    let menuButton: Gtk.MenuButton

    let powerModeRevealer: Gtk.Revealer
    let accentColorRevealer: Gtk.Revealer

    const halfQuickSettingsWidth = Dimensions.quickSettingsWidth / 2 - Dimensions.normalSpacing;

    const lifecycle = createLifecycle()

    lifecycle.onStop(() => {
        powerModeRevealer.revealChild = false;
    })

    onCleanup(() => {
        lifecycle.dispose()
    })

    return (
        <box
            spacing={Dimensions.smallSpacing}
            halign={Gtk.Align.END}
        >
            <menubutton $={(self: Gtk.MenuButton) => (menuButton = self)}>
                <box spacing={Dimensions.normalSpacing}>
                    <BrightnessIcon brightness={brightness}/>

                    <AudioIcon wp={wp}/>

                    <BluetoothIcon bluetooth={bluetooth}/>

                    <NetworkIcon/>

                    <DoNotDisturbIcon notifd={notifd}/>

                    <PowerModeIcon powerProfiles={powerprofiles}/>

                    <BatteryIcon battery={battery}/>

                    <AvatarIcon/>
                </box>

                <popover
                    onShow={() => lifecycle.start()}
                    onClosed={() => {
                        lifecycle.stop()
                    }}
                >
                    <Adw.Clamp
                        css={`
                            padding: ${Dimensions.normalSpacing}px;
                        `}
                        maximumSize={Dimensions.quickSettingsWidth}
                    >

                        <box
                            spacing={Dimensions.smallSpacing}
                            orientation={Gtk.Orientation.VERTICAL}
                        >
                            <centerbox halign={Gtk.Align.FILL} hexpand>
                                <box $type="start" spacing={Dimensions.smallSpacing}>
                                    <BatteryIcon battery={battery}/>
                                </box>
                                <box $type="end" spacing={Dimensions.smallSpacing}>
                                    <button
                                        onClicked={() => {
                                            menuButton.popdown();
                                            DesktopScriptsLib.execAsync("prelock --scale 5%")
                                                .then(() => shAsync("swaylock -f"))
                                        }}
                                        iconName="system-lock-screen-symbolic"
                                    />
                                    <button
                                        onClicked={() => {
                                            menuButton.popdown();
                                            shAsync("loginctl kill-user gouvinb")
                                        }}
                                        iconName="system-log-out-symbolic"
                                    />
                                    <button
                                        onClicked={() => {
                                            menuButton.popdown();
                                            shAsync("systemctl reboot")
                                        }}
                                        iconName="system-reboot-symbolic"
                                    />
                                    <button
                                        onClicked={() => {
                                            menuButton.popdown();
                                            shAsync("systemctl -i poweroff")
                                        }}
                                        iconName="system-shutdown-symbolic"
                                    />
                                </box>
                            </centerbox>
                            <box spacing={Dimensions.smallSpacing} hexpand>
                                <box
                                    css={`
                                        padding-left: ${Dimensions.normalSpacing}px;
                                    `}>
                                    <AudioQS wp={wp}/>
                                </box>
                            </box>
                            <box spacing={Dimensions.smallSpacing} hexpand>
                                <box
                                    css={`
                                        padding-left: ${Dimensions.normalSpacing}px;
                                    `}>
                                    <BrightnessQS brightness={brightness}/>
                                </box>
                            </box>
                            <box
                                hexpand
                                orientation={Gtk.Orientation.VERTICAL}
                            >

                                <box
                                    css={`
                                        padding: ${Dimensions.smallestSpacing}px 0;
                                    `}
                                    spacing={Dimensions.normalSpacing}>
                                    <AirplaneModeButtonQS
                                        parentLifeCycle={lifecycle}
                                        minWidth={halfQuickSettingsWidth}
                                    />
                                    <PowerModeButtonQS
                                        powerProfiles={powerprofiles}
                                        revealer={() => powerModeRevealer}
                                        onReveal={() => {
                                            accentColorRevealer.revealChild = false
                                        }}
                                        minWidth={halfQuickSettingsWidth}/>
                                </box>
                                <box
                                    css={`
                                        padding: ${Dimensions.smallestSpacing}px 0;
                                    `}
                                >
                                    <PowerModeRevealerQS ref={(self: Gtk.Revealer) => (powerModeRevealer = self)}/>
                                </box>

                                <box
                                    css={`
                                        padding: ${Dimensions.smallestSpacing}px 0;
                                    `}
                                    spacing={Dimensions.normalSpacing}
                                >
                                    <DarkModeButtonQS
                                        parentLifecycle={lifecycle}
                                        minWidth={halfQuickSettingsWidth}/>
                                    <AccentColorButtonQS
                                        revealer={() => accentColorRevealer}
                                        onReveal={() => {
                                            powerModeRevealer.revealChild = false
                                        }}
                                        minWidth={halfQuickSettingsWidth}/>
                                </box>
                                <box
                                    css={`
                                        padding: ${Dimensions.smallestSpacing}px 0;
                                    `}
                                >
                                    <AccentColorRevealerQS
                                        parentLifecycle={lifecycle}
                                        ref={(self: Gtk.Revealer) => (accentColorRevealer = self)}/>
                                </box>

                                <box
                                    css={`
                                        padding: ${Dimensions.smallestSpacing}px 0;
                                    `}
                                    spacing={Dimensions.normalSpacing}
                                >
                                    <DoNotDisturbButtonQS
                                        notifd={notifd}
                                        minWidth={halfQuickSettingsWidth}
                                    />
                                    <BluetoothButtonQS minWidth={halfQuickSettingsWidth}/>
                                </box>
                            </box>
                        </box>

                    </Adw.Clamp>
                </popover>
            </menubutton>
        </box>
    )
}
