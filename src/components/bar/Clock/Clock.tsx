import {Accessor, For, onCleanup, With} from "ags"
import {Gtk} from "ags/gtk4"
import {createPoll} from "ags/time"
import GLib from "gi://GLib"
import Adw from "gi://Adw"
import {notifications, setNotifications} from "../../../app/Notifications";
import Notification from "../../notifications/Notification";
import {DataTimePopover} from "./DataTimePopover/DateTimePopover";
import {Dimensions} from "../../../lib/ui/Diemensions";
import {createLifecycle} from "../../../lib/Lifecyle";

export function Clock(
    {
        format = "%a %d %b %Y %H:%M:%S",
        popoverRequestHeight
    }: {
        format?: string,
        popoverRequestHeight: number,
    }
) {
    const rawDateTime: Accessor<GLib.DateTime> = createPoll(
        GLib.DateTime.new_now_local(),
        1000,
        () => GLib.DateTime.new_now_local(),
    )
    const popoverLifecycle = createLifecycle()

    const dateTime = rawDateTime.as((data) => data.format(format)!.capitalize())

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
                                    <For each={notifications}>
                                        {(notification) => <Notification
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
