import AstalNotifd from "gi://AstalNotifd"
import DoNotDisturbIcon from "../icons/DoNotDisturb"

export default function DoNotDisturbButtonQS(
    {minWidth}: { minWidth: number },
) {
    const notifd = AstalNotifd.get_default()

    function toggleDnd() {
        notifd.dontDisturb = !notifd.dontDisturb
    }

    return (
        <togglebutton
            css={`
                min-width: ${minWidth}px;
            `}
            active={notifd.dontDisturb}
            onClicked={() => toggleDnd()}
        >
            <box spacing={8}>
                <DoNotDisturbIcon/>
                <label label={"Do not disturb"}/>
            </box>
        </togglebutton>
    );
}
