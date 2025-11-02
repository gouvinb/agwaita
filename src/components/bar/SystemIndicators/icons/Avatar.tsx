import GLib from "gi://GLib"
import {Gtk} from "ags/gtk4"
import {createState} from "ags"

export default function AvatarIcon() {
    const userName = GLib.get_user_name()
    const avatarPath = `/var/lib/AccountsService/icons/${userName}`

    const [hasAvatar, setHasAvatar] = createState<boolean>(false)

    // Vérifier si l'avatar existe
    function checkAvatarExists(): boolean {
        try {
            return GLib.file_test(avatarPath, GLib.FileTest.EXISTS)
        } catch {
            return false
        }
    }

    setHasAvatar(checkAvatarExists())

    // Si l'avatar existe, afficher l'image
    if (hasAvatar.get()) {
        return (
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
        )
    }

    // Sinon, afficher la première lettre sur fond accent
    const firstLetter = userName.charAt(0).toUpperCase()

    return (
        <box
            css={`
                min-width: 18px;
                min-height: 16px;
                border-radius: 50%;
                background: var(--accent-bg-color);
                padding: 2px;
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
                hexpand={true}
                vexpand={true}
            />
        </box>
    );
}
