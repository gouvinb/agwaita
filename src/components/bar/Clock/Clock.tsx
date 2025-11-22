import {Accessor, createState, For, onCleanup, With} from "ags"
import {Gtk} from "ags/gtk4"
import {createPoll} from "ags/time"
import GLib from "gi://GLib"
import Adw from "gi://Adw"
import AstalNotifd from "gi://AstalNotifd"
import Notification from "../../notifications/Notification"
import {DataTimePopover} from "./DataTimePopover/DateTimePopover"
import {Dimensions} from "../../../lib/ui/Dimensions"
import {createLifecycle} from "../../../lib/Lifecyle"
import "../../../lib/extension/String"
import Agenda from "../../../services/Agenda"

interface ClockProps {
    notifd: AstalNotifd.Notifd,
    agenda: Agenda,
    format?: string,
    popoverRequestHeight: number,
}

export function Clock(
    {
        notifd,
        agenda,
        format = "%a %d %b %Y %H:%M:%S",
        popoverRequestHeight,
    }: ClockProps
) {
    const rawDateTime: Accessor<GLib.DateTime> = createPoll(
        GLib.DateTime.new_now_local(),
        1000,
        () => GLib.DateTime.new_now_local(),
    )

    const dateTime = rawDateTime.as((data) => data.format(format)!.capitalize())

    const [notifications, setNotifications] = createState(
        new Array<AstalNotifd.Notification>(),
    )

    setNotifications(
        notifd.get_notifications()
            .sort((a, b) => b.get_time() - a.get_time())
    )

    let notifiedHandler: number | null
    let resolvedHandler: number | null

    const popoverLifecycle = createLifecycle()
    popoverLifecycle.onStart(() => {
        notifiedHandler = notifd.connect("notified", (_, id, replaced) => {
            const notifications = notifd.get_notifications()
            const newNotifList = (value: AstalNotifd.Notification[]) => {
                if (replaced && notifications.some((n) => n.id === id)) {
                    return value
                        .map((n) => n.id === id ? notification : n)
                        .filter((n) => n != null)
                        .sort((a, b) => b.get_time() - a.get_time())
                } else {
                    return [notification, ...value]
                        .filter((n) => n != null)
                        .sort((a, b) => b.get_time() - a.get_time())
                }
            }

            const notification = notifd.get_notification(id)

            setNotifications(newNotifList)
        })
        resolvedHandler = notifd.connect("resolved", (_, id) => {
            const notificationsResolved = (value: AstalNotifd.Notification[]) => {
                return value
                    .filter((n) => n.id !== id)
                    .sort((a, b) => b.get_time() - a.get_time())
            }

            setNotifications(notificationsResolved)
        })
    })
    popoverLifecycle.onStop(() => {
        if (notifiedHandler) {
            notifd.disconnect(notifiedHandler)
            notifiedHandler = null
        }
        if (resolvedHandler) {
            notifd.disconnect(resolvedHandler)
            resolvedHandler = null
        }
    })

    onCleanup(() => {
        popoverLifecycle.dispose()
    })

    return (
        <menubutton
            label={dateTime}
        >
            <popover
                css_classes={["shared-popover"]}
                heightRequest={popoverRequestHeight}
                onShow={() => {
                    setNotifications(notifd.get_notifications())
                    popoverLifecycle.start()
                }}
                onClosed={() => {
                    popoverLifecycle.stop()
                }}
            >
                <box orientation={Gtk.Orientation.HORIZONTAL} spacing={Dimensions.smallSpacing}>
                    <scrolledwindow
                        propagateNaturalWidth
                        propagateNaturalHeight
                        heightRequest={popoverRequestHeight}
                        widthRequest={Dimensions.notificationWidth + 40}
                        max_content_height={popoverRequestHeight}
                        max_content_width={Dimensions.notificationWidth}
                    >
                        <With value={notifications}>
                            {(notificationList) => notificationList.length > 0 && (
                                <box
                                    css={`
                                        padding: ${Dimensions.normalSpacing}px;
                                    `}
                                    orientation={Gtk.Orientation.VERTICAL}
                                    spacing={Dimensions.smallSpacing}
                                >
                                    <For each={notifications.as((n) => n.slice(0, 5))}>
                                        {(notification: AstalNotifd.Notification) => <Notification
                                            isOverlay={false}
                                            init={
                                                (n) => {
                                                    const timeout_duration = n.expire_timeout
                                                    if (timeout_duration > 0) {
                                                        const timeoutId = setTimeout(
                                                            () => {
                                                                setNotifications((value) => {
                                                                    return value.filter((notif) => notif.id !== n.id)
                                                                })
                                                            },
                                                            timeout_duration
                                                        )

                                                        return () => {
                                                            if (timeoutId) {
                                                                clearTimeout(timeoutId)
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            notification={notification}
                                        />}
                                    </For>
                                </box>
                            ) || (
                                <box
                                    css={`
                                        padding: ${Dimensions.normalSpacing}px;
                                    `}
                                    hexpand
                                    heightRequest={popoverRequestHeight}
                                >
                                    <box
                                        orientation={Gtk.Orientation.VERTICAL}
                                        spacing={Dimensions.smallSpacing}
                                        halign={Gtk.Align.CENTER}
                                        valign={Gtk.Align.CENTER}
                                    >
                                        <image
                                            iconName={"notifications-disabled-symbolic"}
                                            iconSize={Gtk.IconSize.LARGE}
                                            hexpand
                                        />
                                        <label
                                            css={`
                                                font-size: larger;
                                                font-weight: bold;
                                            `}
                                            hexpand
                                            label={"No notifications"}
                                        />
                                    </box>
                                </box>
                            )}
                        </With>
                    </scrolledwindow>

                    <Gtk.Separator orientation={Gtk.Orientation.VERTICAL}/>

                    <box>
                        <Adw.Clamp
                            css={`
                                padding: ${Dimensions.normalSpacing}px;
                            `}
                            orientation={Gtk.Orientation.HORIZONTAL}
                            widthRequest={Dimensions.notificationWidth + 24}
                            maximumSize={Dimensions.notificationWidth + 24}
                        >
                            <DataTimePopover
                                agenda={agenda}
                                parentLifecycle={popoverLifecycle}
                                popoverRequestHeight={popoverRequestHeight}
                            />
                        </Adw.Clamp>
                    </box>
                </box>
            </popover>
        </menubutton>
    )
}
