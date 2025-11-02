import {Gtk} from "ags/gtk4"
import PowerModeIcon from "../icons/PowerMode"

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
            <box spacing={8}>
                <PowerModeIcon/>
                <label label={"Power mode"}/>
                <box hexpand/>
                <image
                    iconName={"pan-end-symbolic"}
                    iconSize={Gtk.IconSize.NORMAL}
                />
            </box>
        </button>
    )
}
