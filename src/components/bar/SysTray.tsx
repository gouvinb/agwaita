import {createBinding, For, onCleanup} from "ags"
import {Gtk} from "ags/gtk4"
import Tray from "gi://AstalTray"
import {Dimensions} from "../../lib/ui/Diemensions";

export default function SysTray() {
    const tray = Tray.get_default()
    const items = createBinding(tray, "items")

    const init = (btn: Gtk.MenuButton, item: Tray.TrayItem) => {
        btn.menuModel = item.menuModel
        btn.insert_action_group("dbusmenu", item.actionGroup)
        item.connect("notify::action-group", () => {
            btn.insert_action_group("dbusmenu", item.actionGroup)
        })
    }

    onCleanup(() => {
    })

    return (
        <box spacing={Dimensions.smallSpacing}>
            <For each={items}>
                {(item) => (
                    <menubutton
                        $={(self) => init(self, item)}
                        tooltip_text={item.tooltipText}
                    >
                        <image gicon={createBinding(item, "gicon")}/>
                    </menubutton>
                )}
            </For>
        </box>
    )
}
