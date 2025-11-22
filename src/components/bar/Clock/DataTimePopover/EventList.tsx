import {Gtk} from "ags/gtk4"
import {Accessor, For, With} from "ags"
import "../../../../lib/extension/GLibDateTime"
import "../../../../lib/extension/String"
import {CalendarEvent} from "../../../../services/Agenda"
import {Dimensions} from "../../../../lib/ui/Dimensions"

interface EventListProps {
    title?: Accessor<string>,
    events: Accessor<CalendarEvent[]>
    predicate?: (eventList: CalendarEvent[]) => boolean,
}

export function EventList(
    {
        title = new Accessor<string>(() => "Today"),
        events,
        predicate = (eventList) => eventList.length > 0
    }: EventListProps
) {

    return (
        <With value={events}>
            {(eventList) => predicate(eventList) && (
                <box
                    css={`
                        padding-left: ${Dimensions.normalSpacing}px;
                    `}
                    orientation={Gtk.Orientation.VERTICAL}
                    spacing={Dimensions.smallSpacing}
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
                                        border-top-left-radius: ${Dimensions.smallSpacing}px;
                                        border-bottom-left-radius: ${Dimensions.smallSpacing}px;
                                        padding: ${Dimensions.smallestSpacing}px;
                                    `}/>
                                <box
                                    css={`
                                        background: alpha(${entry.color ?? "--accent-color"}, 0.25);
                                        border-top-right-radius: ${Dimensions.smallSpacing}px;
                                        border-bottom-right-radius: ${Dimensions.smallSpacing}px;
                                        padding: ${Dimensions.smallSpacing}px;
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
