import {Gtk} from "ags/gtk4";

export default function AccentColorIcon() {
    return (
        <image
            css={`
                color: var(--accent-color);
            `}
            iconName={"preferences-color-symbolic"}
            iconSize={Gtk.IconSize.NORMAL}
        />
    )
}
