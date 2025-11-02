import {Gtk} from "ags/gtk4";
import {Accessor, For, With} from "ags";
import {CalendarEvent} from "./Agenda";
import "../../../../lib/extension/GLibDateTime"
import "../../../../lib/extension/String"

export function EventList(
    {
        title = new Accessor<string>(() => "Today"),
        events,
        predicate = (eventList) => eventList.length > 0
    }: {
        title?: Accessor<string>,
        events: Accessor<CalendarEvent[]>
        predicate?: (eventList: CalendarEvent[]) => boolean,
    }
) {

    return (
        <With value={events}>
            {(eventList) => predicate(eventList) && (
                <box
                    css={`
                        padding-left: 8px;
                    `}
                    orientation={Gtk.Orientation.VERTICAL}
                    spacing={4}
                >
                    <label
                        css={`
                            font-weight: bold;
                        `}
                        halign={Gtk.Align.START}
                        label={title}
                    />
                    <For each={events}>
                        {(entry) => (
                            <box>
                                <box
                                    css={`
                                        background: ${entry.color ?? "--accent-color"};
                                        border-top-left-radius: 4px;
                                        border-bottom-left-radius: 4px;
                                        padding: 2px;
                                    `}/>
                                <box
                                    css={`
                                        background: alpha(${entry.color ?? "--accent-color"}, 0.25);
                                        border-top-right-radius: 4px;
                                        border-bottom-right-radius: 4px;
                                        padding: 4px;
                                    `}
                                    orientation={Gtk.Orientation.VERTICAL}
                                >
                                    <label
                                        css={`
                                            font-weight: bold;
                                        `}
                                        hexpand
                                        wrap
                                        halign={Gtk.Align.START}
                                        label={`${entry.summary}`}/>
                                    {(entry.isAllDay && (
                                        <label
                                            css={`
                                                font-size: small;
                                            `}
                                            hexpand
                                            halign={Gtk.Align.START}
                                            label={`All Day`}/>
                                    )) || (
                                        <label
                                            css={`
                                                font-size: small;
                                            `}
                                            halign={Gtk.Align.START}
                                            label={`${entry.start.format("%H:%M")} - ${entry.end.format("%H:%M")}`}/>
                                    )}
                                </box>
                            </box>
                        )}
                    </For>
                </box>
            )}
        </With>
    )
}
