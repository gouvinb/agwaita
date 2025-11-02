import {createState} from "ags"
import PowerProfiles from "gi://AstalPowerProfiles"

export default function PowerModeIcon() {
    const powerprofiles = PowerProfiles.get_default()

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

    powerprofiles.connect("notify::active-profile", ({activeProfile}) => {
        updateIcon(activeProfile)
    })

    updateIcon(powerprofiles.activeProfile)

    return (
        <image
            iconName={icon}
            pixelSize={16}
        />
    )
}
