import {createBinding, createEffect, createState} from "ags"
import {Gtk} from "ags/gtk4"
import AstalNotifd from "gi://AstalNotifd"
import {Accessor} from "gnim"

interface DotNotDisturbIconProps {
    notifd: AstalNotifd.Notifd
}

export default function DoNotDisturbIcon({notifd}: DotNotDisturbIconProps) {
    const dontDisturb: Accessor<boolean> = createBinding(notifd, "dontDisturb")
    const [iconName, setIconName] = createState<string>("org.gnome.Settings-notifications-symbolic")

    createEffect(() => {
        const newIcon = dontDisturb()
            ? "notifications-disabled-symbolic"
            : "org.gnome.Settings-notifications-symbolic"

        if (iconName.peek() !== newIcon) {
            setIconName(newIcon)
        }
    })

    return (
        <image
            iconName={iconName}
            iconSize={Gtk.IconSize.NORMAL}
        />
    )
}
