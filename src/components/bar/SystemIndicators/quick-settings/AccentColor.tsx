import {Gtk} from "ags/gtk4"
import AccentColorIcon from "../icons/AccentColor"
import {Dimensions} from "../../../../lib/ui/Dimensions";

interface AccentColorButtonQSProps {
    revealer: () => Gtk.Revealer,
    onReveal: () => void,
    minWidth: number,
}

export default function AccentColorButtonQS(
    {
        revealer,
        onReveal,
        minWidth,
    }: AccentColorButtonQSProps
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
                <AccentColorIcon/>
                <label label={"Accent color"}/>
                <box hexpand/>
                <image
                    iconName={"pan-down-symbolic"}
                    iconSize={Gtk.IconSize.NORMAL}
                />
            </box>
        </button>
    )
}
