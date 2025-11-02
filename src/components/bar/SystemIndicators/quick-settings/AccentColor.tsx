import {Gtk} from "ags/gtk4"
import AccentColorIcon from "../icons/AccentColor"

export default function AccentColorButtonQS(
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
                <AccentColorIcon/>
                <label label={"Accent color"}/>
                <box hexpand/>
                <image
                    iconName={"pan-end-symbolic"}
                    iconSize={Gtk.IconSize.NORMAL}
                />
            </box>
        </button>
    )
}
