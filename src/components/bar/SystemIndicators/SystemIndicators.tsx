import {Gtk} from "ags/gtk4"
import BrightnessIcon from "./icons/Brightness"
import AudioIcon from "./icons/Audio"
import BluetoothIcon from "./icons/Bluetooth"
import DoNotDisturbIcon from "./icons/DoNotDisturb"
import NetworkIcon from "./icons/Network"
import PowerModeIcon from "./icons/PowerMode";
import BatteryIcon from "./icons/Battery";
import AvatarIcon from "./icons/Avatar";
import {shAsync} from "../../../lib/ExternalCommand";
import DesktopScriptsLib from "../../../lib/ExternalCommand";
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

export function SystemIndicators() {
    let menuButton: Gtk.MenuButton

    let powerModeRevealer: Gtk.Revealer
    let accentColorRevealer: Gtk.Revealer

    const reqWidth = 320;
    const vColumnSpacing = 8;

    const demiWidth = reqWidth / 2 - vColumnSpacing;

    return (
        <box spacing={4} halign={Gtk.Align.END}>
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
                    <box
                        css={`
                            padding: 8px;
                        `}
                        widthRequest={reqWidth}
                        spacing={4}
                        orientation={Gtk.Orientation.VERTICAL}
                    >
                        <centerbox halign={Gtk.Align.FILL} hexpand={true}>
                            <box $type="start" spacing={4}>
                                <BatteryIcon/>
                            </box>
                            <box $type="end" spacing={4}>
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
                        <box spacing={4} hexpand={true}>
                            <box
                                css={`
                                    padding-left: 8px;
                                `}>
                                <AudioQS/>
                            </box>
                        </box>
                        <box spacing={4} hexpand={true}>
                            <box
                                css={`
                                    padding-left: 8px;
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
                                    padding: 2px 0;
                                `}
                                spacing={vColumnSpacing}>
                                <AirplaneModeButtonQS minWidth={demiWidth}/>
                                <PowerModeButtonQS
                                    revealer={() => powerModeRevealer}
                                    onReveal={() => {
                                        accentColorRevealer.revealChild = false
                                    }}
                                    minWidth={demiWidth}/>
                            </box>
                            <box
                                css={`
                                    padding: 2px 0;
                                `}
                            >
                                <PowerModeRevealerQS ref={(self: Gtk.Revealer) => (powerModeRevealer = self)}/>
                            </box>

                            <box
                                css={`
                                    padding: 2px 0;
                                `}
                                spacing={vColumnSpacing}
                            >
                                <DarkModeButtonQS minWidth={demiWidth}/>
                                <AccentColorButtonQS
                                    revealer={() => accentColorRevealer}
                                    onReveal={() => {
                                        powerModeRevealer.revealChild = false
                                    }}
                                    minWidth={demiWidth}/>
                            </box>
                            <box
                                css={`
                                    padding: 2px 0;
                                `}
                            >
                                <AccentColorRevealerQS ref={(self: Gtk.Revealer) => (accentColorRevealer = self)}/>
                            </box>

                            <box
                                css={`
                                    padding: 2px 0;
                                `}
                                spacing={vColumnSpacing}
                            >
                                <DoNotDisturbButtonQS minWidth={demiWidth}/>
                                <BluetoothButtonQS minWidth={demiWidth}/>
                            </box>
                        </box>
                    </box>
                </popover>
            </menubutton>
        </box>
    )
}
