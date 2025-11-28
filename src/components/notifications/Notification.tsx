import Gtk from "gi://Gtk"
import Gdk from "gi://Gdk"
import GLib from "gi://GLib"
import Adw from "gi://Adw"
import Pango from "gi://Pango"
import AstalNotifd from "gi://AstalNotifd"
import {Dimensions} from "../../lib/ui/Dimensions";
import {Shapes} from "../../lib/ui/Shapes";


interface NotificationProps {
    notification: AstalNotifd.Notification,
    isOverlay: boolean,
    init: ((notification: AstalNotifd.Notification) => void),
}

export default function Notification(
    {
        notification,
        isOverlay,
        init
    }: NotificationProps
) {
    const cssClasses = getCssClasses(notification, isOverlay)


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

    function getCssClasses(n: AstalNotifd.Notification, isOverlay: boolean) {
        const {LOW, NORMAL, CRITICAL} = AstalNotifd.Urgency
        const result = ["shared-notification"]

        if (isOverlay) {
            result.push("overlay")
        } else {
            result.push("card")
        }

        switch (n.urgency) {
            case LOW:
                result.push("low-priority")
                break
            case CRITICAL:
                result.push("critical-priority")
                break
            case NORMAL:
            default:
                result.push("normal-priority")
                break
        }
        return result
    }

    return (
        <Adw.Clamp
            css={`
                padding: ${Dimensions.normalSpacing}px;
                border-spacing: ${Dimensions.noSpacing}px;
                border-radius: ${Shapes.windowRadius}px;
                background-clip: padding-box;
            `}
            cssClasses={cssClasses}
            maximumSize={Dimensions.notificationWidth}
        >
            <box
                css={`
                    padding: ${Dimensions.smallSpacing}px;
                `}
                spacing={Dimensions.smallestSpacing}
                $={() => init(notification)}
                widthRequest={Dimensions.notificationWidth}
                orientation={Gtk.Orientation.VERTICAL}
            >
                <box
                    css={`
                        padding: ${Dimensions.smallSpacing}px;
                    `}
                    spacing={Dimensions.smallSpacing}
                >
                    {(notification.appIcon || isIcon(notification.desktopEntry)) && (
                        <image
                            marginEnd={Dimensions.semiBigSpacing}
                            iconName={notification.appIcon || notification.desktopEntry || "application-x-executable"}
                            iconSize={Gtk.IconSize.NORMAL}
                        />
                    )}
                    <label
                        css={`
                            font-style: italic;
                        `}
                        halign={Gtk.Align.START}
                        valign={Gtk.Align.FILL}
                        ellipsize={Pango.EllipsizeMode.END}
                        label={notification.appName || "Unknown"}
                    />
                    <label
                        css={`
                            font-size: x-small;
                        `}
                        hexpand
                        halign={Gtk.Align.END}
                        valign={Gtk.Align.FILL}
                        label={time(notification.time)}
                    />
                    <button
                        marginStart={Dimensions.semiBigSpacing}
                        onClicked={() => notification.dismiss()}
                        iconName="window-close-symbolic"
                    />
                </box>
                <Gtk.Separator/>
                <box
                    css={`
                        padding: ${Dimensions.smallSpacing}px;
                    `}
                    spacing={Dimensions.semiBigSpacing}
                >
                    {
                        (notification.image && isIcon(notification.image) && (
                            <image
                                iconName={notification.image}
                                halign={Gtk.Align.CENTER}
                                valign={Gtk.Align.START}
                                iconSize={Gtk.IconSize.LARGE}
                                marginTop={Dimensions.normalSpacing}

                            />
                        )) || (notification.image && fileExists(notification.image) && (
                            <image
                                file={notification.image}
                                halign={Gtk.Align.CENTER}
                                valign={Gtk.Align.START}
                                iconSize={Gtk.IconSize.LARGE}
                                marginTop={Dimensions.normalSpacing}
                            />
                        ))
                    }
                    <box
                        spacing={Dimensions.smallSpacing}
                        orientation={Gtk.Orientation.VERTICAL}
                        marginTop={Dimensions.smallSpacing}
                    >
                        <label
                            css={`
                                font-weight: bold;
                            `}
                            halign={Gtk.Align.START}
                            label={notification.summary}
                            ellipsize={Pango.EllipsizeMode.END}
                        />
                        {notification.body && (<label
                                css={`
                                    font-size: small;
                                `}
                                halign={Gtk.Align.START}
                                wrap
                                useMarkup
                                label={
                                    // notification.body
                                    //         .replace("<", "&lt;")
                                    //         .replace(">", "&gt;")
                                    //         .replace("&", "&amp;")
                                    //         .replace(/&/g, "&amp;")
                                    //         .replace("<br/>", "\n")
                                    GLib.markup_escape_text(notification.body, -1)
                                }
                            />
                        )}
                    </box>
                </box>
                {notification.actions.length > 0 && (
                    <box
                        css={`
                            padding: ${Dimensions.smallSpacing}px;
                        `}
                        spacing={Dimensions.smallSpacing}
                    >
                        {notification.actions.map(({label, id}) => (
                            <button
                                hexpand
                                onClicked={() => notification.invoke(id)}
                                label={label} halign={Gtk.Align.CENTER}
                            />
                        ))}
                    </box>
                )}
            </box>
        </Adw.Clamp>
    )
}
