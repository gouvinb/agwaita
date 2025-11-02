import Gtk from "gi://Gtk?version=4.0"
import Gdk from "gi://Gdk?version=4.0"
import GLib from "gi://GLib?version=2.0"
import Adw from "gi://Adw?version=1"
import Pango from "gi://Pango?version=1.0"
import AstalNotifd from "gi://AstalNotifd?version=0.1"


export const notificationWidth = 320

function isIcon(icon?: string | null) {
    const iconTheme = Gtk.IconTheme.get_for_display(Gdk.Display.get_default()!)
    return icon && iconTheme.has_icon(icon)
}

function fileExists(path: string) {
    return GLib.file_test(path, GLib.FileTest.EXISTS)
}

function time(time: number, format = "%H:%M") {
    return GLib.DateTime.new_from_unix_local(time).format(format)!
}

function getUrgencyColor(n: AstalNotifd.Notification) {
    const {LOW, NORMAL, CRITICAL} = AstalNotifd.Urgency
    switch (n.urgency) {
        case LOW:
            return "transparent"
        case CRITICAL:
            return "@error_color"
        case NORMAL:
        default:
            return "@accent_color"
    }
}

interface NotificationProps {
    notification: AstalNotifd.Notification,
    init: ((notification: AstalNotifd.Notification) => void),
}

export default function Notification({notification: n, init}: NotificationProps) {
    const borderColor = getUrgencyColor(n)

    return (
        <Adw.Clamp
            css={`
                padding: 8px;
                border-spacing: 8px;
                border-radius: 16px;
                border: 2px solid ${borderColor};
                background-color: var(--dialog-bg-color);
                background-clip: padding-box;
                color: var(--dialog-fg-color);
            `}
            maximumSize={notificationWidth}
        >
            <box
                css={`
                    padding: 4px;
                `}
                spacing={4}
                $={() => init(n)}
                widthRequest={notificationWidth}
                orientation={Gtk.Orientation.VERTICAL}
            >
                <box
                    css={`
                        padding: 4px;
                    `}
                    spacing={4}
                >
                    {(n.appIcon || isIcon(n.desktopEntry)) && (
                        <image
                            marginEnd={12}
                            iconName={n.appIcon || n.desktopEntry || "application-x-executable"}
                            iconSize={Gtk.IconSize.LARGE}
                        />
                    )}
                    <label
                        css={`
                            font-style: italic;
                        `}
                        halign={Gtk.Align.START}
                        valign={Gtk.Align.FILL}
                        ellipsize={Pango.EllipsizeMode.END}
                        label={n.appName || "Unknown"}
                    />
                    <label
                        css={`
                            font-size: x-small;
                        `}
                        hexpand
                        halign={Gtk.Align.END}
                        valign={Gtk.Align.FILL}
                        label={time(n.time)}
                    />
                    <button
                        marginStart={12}
                        onClicked={() => n.dismiss()}
                        iconName="window-close-symbolic"
                    />
                </box>
                <Gtk.Separator/>
                <box
                    css={`
                        padding: 4px;
                    `}
                    spacing={12}
                >
                    {n.image && fileExists(n.image) && (
                        <image valign={Gtk.Align.FILL}/>
                    )}
                    {n.image && isIcon(n.image) && (
                        <box valign={Gtk.Align.FILL}>
                            <image
                                iconName={n.image}
                                halign={Gtk.Align.CENTER}
                                valign={Gtk.Align.CENTER}
                            />
                        </box>
                    )}
                    <box
                        css={`
                            padding: 4px;
                        `}
                        spacing={4}
                        orientation={Gtk.Orientation.VERTICAL}
                    >
                        <label
                            css={`
                                font-weight: bold;
                            `}
                            halign={Gtk.Align.START}
                            xalign={0}
                            label={n.summary}
                            ellipsize={Pango.EllipsizeMode.END}
                        />
                        {n.body && (
                            <label
                                css={`
                                    font-size: small;
                                `}
                                wrap
                                useMarkup
                                halign={Gtk.Align.START}
                                xalign={0}
                                justify={Gtk.Justification.LEFT}
                                // label={escapeMarkup(n.body)}
                                label={n.body}
                            />
                        )}
                    </box>
                </box>
                {n.actions.length > 0 && (
                    <box
                        css={`
                            padding: 4px;
                        `}
                        spacing={4}
                    >
                        {n.actions.map(({label, id}) => (
                            <button
                                hexpand
                                onClicked={() => n.invoke(id)}
                                label={label} halign={Gtk.Align.CENTER}
                            />
                        ))}
                    </box>
                )}
            </box>
        </Adw.Clamp>
    )
}
