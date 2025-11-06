import {Gtk} from "ags/gtk4"
import PowerModeIcon from "../icons/PowerMode"
import {Dimensions} from "../../../../lib/ui/Diemensions";

export default function PowerModeButtonQS(
    {revealer, onReveal, minWidth}: { revealer: () => Gtk.Revealer, onReveal: () => void, minWidth: number },
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
                <PowerModeIcon/>
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
