import {createState} from "ags"
import PowerProfiles from "gi://AstalPowerProfiles"
import {Gtk} from "ags/gtk4";

interface PowerModeIconProps {
    powerProfiles: PowerProfiles.PowerProfiles
}

export default function PowerModeIcon({powerProfiles}: PowerModeIconProps) {
    const [icon, setIcon] = createState<string>("org.gnome.Settings-power-symbolic")

    function updateIcon(activeProfile: string) {
        switch (activeProfile) {
            case "power-saver":
                setIcon("power-profile-power-saver-symbolic")
                return
            case "performance":
                setIcon("power-profile-performance-symbolic")
                return
            case "balanced":
            default:
                setIcon("power-profile-balanced-symbolic")
                return
        }
    }

    powerProfiles.connect("notify::active-profile", ({activeProfile}) => {
        updateIcon(activeProfile)
    })

    updateIcon(powerProfiles.activeProfile)

    return (
        <image
            iconName={icon}
            iconSize={Gtk.IconSize.NORMAL}
        />
    )
}
