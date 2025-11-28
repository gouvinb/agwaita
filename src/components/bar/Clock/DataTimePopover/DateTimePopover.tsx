import {Accessor, createEffect, createState, onCleanup} from "ags"
import {Gtk} from "ags/gtk4"
import {interval, Timer} from "ags/time"
import GLib from "gi://GLib"
import Agenda, {CalendarEvent} from "../../../../services/Agenda"
import {Calendar} from "./Calendar"
import {EventList} from "./EventList"
import {DateTimeExt} from "../../../../lib/extension/GLibDateTime"
import {Dimensions} from "../../../../lib/ui/Dimensions"
import {Lifecycle} from "../../../../lib/Lifecyle"

interface DateTimePopoverProps {
    agenda: Agenda,
    parentLifecycle: Lifecycle,
    popoverRequestHeight: number,
}

export function DataTimePopover(
    {
        agenda,
        parentLifecycle,
        popoverRequestHeight,
    }: DateTimePopoverProps,
) {
    const [rawDateTime, setRawDateTime] = createState(GLib.DateTime.new_now_local())

    let rawDateTimeTimer: Timer | null = null

    const [calendarSelectedDate, setCalendarSelectedDate] = createState<GLib.DateTime>(rawDateTime.peek())
    const calendarSelectedDateFormatted = calendarSelectedDate.as((date) => date.format("%A %d %b %Y")!.capitalize())

    const [eventsAccessor, setEvents] = createState<CalendarEvent[]>([])

    const todayEventsAccessor: Accessor<CalendarEvent[]> = eventsAccessor.as((events) =>
        events.filter((event) => event.start.format("%Y%m%d")! === (rawDateTime?.peek().format("%Y%m%d") ?? ""))
    )


    const selectedDayEventsAccessor: Accessor<CalendarEvent[]> = calendarSelectedDate.as((date: GLib.DateTime) =>
        eventsAccessor.peek().filter((event) => event.start.format("%Y%m%d")! === date.format("%Y%m%d")!)
    )

    let eventNotifier: number | null = agenda.connect("notify::events", () => setEvents(agenda.events))

    let calendar: Gtk.Calendar

    const markCalendar = () => {
        calendar.clear_marks();

        const currentYear = calendar.get_date().get_year();
        const currentMonth = calendar.get_date().get_month();

        eventsAccessor().forEach((event) => {
            const dateStr = event.start.format("%Y%m%d")!;
            const year = parseInt(dateStr.substring(0, 4));
            const month = parseInt(dateStr.substring(4, 6));
            const day = parseInt(dateStr.substring(6, 8));

            if (year === currentYear && month === currentMonth) {
                calendar.mark_day(day);
            }
        });
    }

    const updateDaySelected = (calendar: Gtk.Calendar) => setCalendarSelectedDate(calendar.get_date())
    parentLifecycle.onStart(() => {
        rawDateTimeTimer = interval(
            1000,
            () => setRawDateTime(GLib.DateTime.new_now_local()),
        )
        if (eventNotifier != null) {
            eventNotifier = agenda.connect("notify::events", () => setEvents(agenda.events))
        }
    })
    parentLifecycle.onStop(() => {
        rawDateTimeTimer?.cancel()
        rawDateTimeTimer = null

        if (eventNotifier != null) {
            agenda.disconnect(eventNotifier)
        }
        eventNotifier = null
    })

    onCleanup(() => {
    })

    createEffect(markCalendar)

    return (
        <box
            orientation={Gtk.Orientation.VERTICAL}
            spacing={Dimensions.smallSpacing}
        >
            <Calendar
                parentLifecycle={parentLifecycle}
                ref={(instance) => {
                    calendar = instance
                }}
                markCalendar={markCalendar}
                updateDaySelected={updateDaySelected}
            />

            <scrolledwindow
                propagateNaturalWidth
                propagateNaturalHeight
                hexpand
                vexpand
                widthRequest={Dimensions.notificationWidth + 24}
                max_content_height={popoverRequestHeight / 2 + 24}
            >
                <box orientation={Gtk.Orientation.VERTICAL}>
                    <box>
                        <EventList events={todayEventsAccessor}/>
                    </box>
                    <box marginTop={8}/>
                    <box>
                        <EventList
                            title={calendarSelectedDateFormatted}
                            events={selectedDayEventsAccessor}
                            predicate={(eventList) => !DateTimeExt.isToday(calendarSelectedDate.peek()) && eventList.length > 0}
                        />
                    </box>
                </box>
            </scrolledwindow>
        </box>
    )
}
