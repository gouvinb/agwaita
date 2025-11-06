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
import AirplaneModeButtonQS from "./quick-settings/AirplaneMode";
import DoNotDisturbButtonQS from "./quick-settings/DoNotDisturb";
import AccentColorButtonQS from "./quick-settings/AccentColor";
import AccentColorRevealerQS from "./quick-settings/revealer/AccentColor";
import {Dimensions} from "../../../lib/ui/Diemensions";

export function SystemIndicators() {
    let menuButton: Gtk.MenuButton

    let powerModeRevealer: Gtk.Revealer
    let accentColorRevealer: Gtk.Revealer

    const halfQuickSettingsWidth = Dimensions.quickSettingsWidth / 2 - Dimensions.normalSpacing;

    return (
        <box
            spacing={Dimensions.smallSpacing}
            halign={Gtk.Align.END}
        >
            <menubutton $={(self: Gtk.MenuButton) => (menuButton = self)}>
                <box spacing={8}>
                    <BrightnessIcon/>

                    <AudioIcon/>

                    <BluetoothIcon/>

                    <NetworkIcon/>

                    <DoNotDisturbIcon/>

                    <PowerModeIcon/>

                    <BatteryIcon/>

                    <AvatarIcon/>
                </box>

                <popover
                    onClosed={() => {
                        powerModeRevealer.revealChild = false;
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
                                    <BatteryIcon/>
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
                                    <AudioQS/>
                                </box>
                            </box>
                            <box spacing={Dimensions.smallSpacing} hexpand>
                                <box
                                    css={`
                                        padding-left: ${Dimensions.normalSpacing}px;
                                    `}>
                                    <BrightnessQS/>
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
                                    spacing={ Dimensions.normalSpacing}>
                                    <AirplaneModeButtonQS minWidth={halfQuickSettingsWidth}/>
                                    <PowerModeButtonQS
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
                                    spacing={ Dimensions.normalSpacing}
                                >
                                    <DarkModeButtonQS minWidth={halfQuickSettingsWidth}/>
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
                                    <AccentColorRevealerQS ref={(self: Gtk.Revealer) => (accentColorRevealer = self)}/>
                                </box>

                                <box
                                    css={`
                                        padding: ${Dimensions.smallestSpacing}px 0;
                                    `}
                                    spacing={ Dimensions.normalSpacing}
                                >
                                    <DoNotDisturbButtonQS minWidth={halfQuickSettingsWidth}/>
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
