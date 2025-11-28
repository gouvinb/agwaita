import app from "ags/gtk4/app"
import {Accessor, For, onCleanup} from "ags"
import {Log} from "../lib/Logger"
import {Astal, Gdk, Gtk} from "ags/gtk4";
import Adw from "gi://Adw";
import {Shapes} from "../lib/ui/Shapes";
import DesktopScriptsLib, {shAsync} from "../lib/ExternalCommand";
import {Dimensions} from "../lib/ui/Dimensions";

interface PowerMenuEntry {
    title: string,
    iconName: string,
    action: () => void,
}

export default function PowerMenu() {
    const powerMenuEntries: Accessor<PowerMenuEntry[]> = new Accessor(
        () => [
            {
                title: "Lock screen",
                iconName: "system-lock-screen-symbolic",
                action: () => {
                    DesktopScriptsLib.execAsync("prelock --scale 5%")
                        .then(() => shAsync("swaylock -f"))
                }
            },
            {
                title: "Log-out",
                iconName: "system-log-out-symbolic",
                action: () => {
                    shAsync("loginctl kill-user gouvinb")
                }
            },
            {
                title: "Reboot",
                iconName: "system-reboot-symbolic",
                action: () => {
                    shAsync("systemctl reboot")

                }
            },
            {
                title: "Shutdown",
                iconName: "system-shutdown-symbolic",
                action: () => {
                    shAsync("systemctl -i poweroff")
                }
            },
        ],
    )

    const {TOP, RIGHT, LEFT, BOTTOM} = Astal.WindowAnchor

    let win: Astal.Window;

    onCleanup(() => {
        win.destroy()
    })

    function applyCssForDeviceRow(position: number) {
        switch (position) {
            case 0:
                return `border-radius: ${Shapes.windowRadius}px ${Shapes.windowRadius}px 0 0;`
            case powerMenuEntries.get().length - 1:
                return `border-radius: 0 0 ${Shapes.windowRadius}px ${Shapes.windowRadius}px;`
            default:
                return ""
        }
    }

    return (
        <window
            $={(self) => win = self}
            name="powermenu.gui"
            application={app}
            layer={Astal.Layer.OVERLAY}
            css={`background: transparent;`}
            anchor={TOP | LEFT | RIGHT | BOTTOM}
            modal
            onShow={() => {
                Log.d("Power menu", "Window opened")
            }}
            onCloseRequest={(self) => {
                Log.d("Power menu", "Window closed")
                self.hide()
                return true
            }}
            title="Power menu"
            keymode={Astal.Keymode.EXCLUSIVE}
        >
            <Gtk.EventControllerKey
                onKeyPressed={({widget}, keyval: number) => {
                    if (keyval === Gdk.KEY_Escape) {
                        widget.hide()
                    }
                }}
            />

            <box
                css={`
                    background-color: var(--window-bg-color);
                    border: 1px solid var(--border-color);
                    border-radius: ${Shapes.windowRadius}px;
                    padding: ${Dimensions.semiBigSpacing}px;
                `}
                orientation={Gtk.Orientation.VERTICAL}
                halign={Gtk.Align.CENTER}
                valign={Gtk.Align.CENTER}
            >
                <Gtk.ListBox
                    css={`
                        border-radius: ${Shapes.windowRadius}px;
                        background-color: transparent;
                    `}
                >
                    <For each={powerMenuEntries}>
                        {(entry, index) => (
                            <Adw.ActionRow
                                name={`power-menu-entry-${index.get()}`}
                                css={`
                                    border-radius: ${Shapes.componentRadius}px;
                                `}
                                title={entry.title}
                                iconName={entry.iconName}
                                activatable
                                onActivated={() => {
                                    win.close()
                                    entry.action()
                                }}
                            />
                        )}
                    </For>
                </Gtk.ListBox>
            </box>
        </window>
    )
}
