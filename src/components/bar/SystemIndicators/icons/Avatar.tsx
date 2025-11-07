import GLib from "gi://GLib"
import {Gtk} from "ags/gtk4"
import {createState, With} from "ags"
import {interval} from "ags/time";
import {Dimensions} from "../../../../lib/ui/Dimensions";

export default function AvatarIcon() {
    const userName = GLib.get_user_name()
    const firstLetter = userName.charAt(0).toUpperCase()

    const avatarPath = `/var/lib/AccountsService/icons/${userName}`

    const [hasAvatar, setHasAvatar] = createState<boolean>(checkAvatarExists())

    function checkAvatarExists(): boolean {
        try {
            return GLib.file_test(avatarPath, GLib.FileTest.EXISTS)
        } catch {
            return false
        }
    }

    interval(1000, () => {
        setHasAvatar(checkAvatarExists())
    })

    return (
        <With value={hasAvatar}>
            {(value) => value && (
                <box
                    css={`
                        border-radius: 50%;
                        background: var(--accent-bg-color);
                    `}
                    halign={Gtk.Align.CENTER}
                    valign={Gtk.Align.CENTER}
                    overflow={Gtk.Overflow.HIDDEN}
                >
                    <image
                        file={avatarPath}
                        pixelSize={16}
                    />
                </box>
            ) || (
                <box
                    css={`
                        min-width: 18px;
                        min-height: 16px;
                        border-radius: 50%;
                        background: var(--accent-bg-color);
                        padding: ${Dimensions.smallestSpacing}px;
                    `}
                    halign={Gtk.Align.CENTER}
                    valign={Gtk.Align.CENTER}
                >
                    <label
                        label={firstLetter}
                        css={`
                            font-family: monospace;
                            font-size: small;
                            font-weight: bold;
                            color: var(--accent-fg-color);
                        `}
                        halign={Gtk.Align.CENTER}
                        valign={Gtk.Align.CENTER}
                        hexpand
                        vexpand
                    />
                </box>
            )}
        </With>
    );
}
