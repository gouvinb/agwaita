import {createBinding, createState} from "ags"
import {Gtk} from "ags/gtk4"
import AstalNotifd from "gi://AstalNotifd"
import {Accessor} from "gnim"

export default function DoNotDisturbIcon() {
    const notifd = AstalNotifd.get_default()

    const dontDisturb: Accessor<boolean> = createBinding(notifd, "dontDisturb")
    const [iconName, setIconName] = createState<string>("org.gnome.Settings-notifications-symbolic")

    function updateIcon() {
        const newIcon = dontDisturb.get()
            ? "notifications-disabled-symbolic"
            : "org.gnome.Settings-notifications-symbolic"

        if (iconName.get() !== newIcon) {
            setIconName(newIcon)
        }
    }

    dontDisturb.subscribe(() => {
        updateIcon()
    })

    updateIcon()

    return (
        <image
            iconName={iconName}
            iconSize={Gtk.IconSize.NORMAL}
        />
    );
}
