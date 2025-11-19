import {Gtk} from "ags/gtk4"
import PowerModeIcon from "../icons/PowerMode"
import PowerProfiles from "gi://AstalPowerProfiles"
import {Dimensions} from "../../../../lib/ui/Dimensions";

interface PowerModeButtonQSProps {
    powerProfiles: PowerProfiles.PowerProfiles
    revealer: () => Gtk.Revealer,
    onReveal: () => void,
    minWidth: number,
}

export default function PowerModeButtonQS(
    {
        powerProfiles,
        revealer,
        onReveal,
        minWidth,
    }: PowerModeButtonQSProps,
) {
    return (
        <button
            css={`
                min-width: ${minWidth}px;
            `}
            onClicked={() => {
                const rev = revealer()
                if (rev) {
                    rev.revealChild = !rev.revealChild
                    if (rev.revealChild) {
                        onReveal()
                    }
                }
            }}
        >
            <box spacing={Dimensions.normalSpacing}>
                <PowerModeIcon powerProfiles={powerProfiles}/>
                <label label={"Power mode"}/>
                <box hexpand/>
                <image
                    iconName={"pan-down-symbolic"}
                    iconSize={Gtk.IconSize.NORMAL}
                />
            </box>
        </button>
    )
}
