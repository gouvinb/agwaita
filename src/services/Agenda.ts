import {interval, Timer} from "ags/time"
import GLib from "gi://GLib"
import EDataServer from "gi://EDataServer"
import ECal from "gi://ECal"
import ICalGLib from "gi://ICalGLib"
import GObject, {getter, register} from "gnim/gobject"
import {Log} from "../lib/Logger";
import {Accessor, createState, Setter} from "ags";

export type CalendarEvent = {
    summary: string
    desc: string
    color?: string
    isAllDay: boolean
    start: GLib.DateTime
    end: GLib.DateTime
}

let instance: Agenda

@register({GTypeName: "AgendaService"})
export default class Agenda extends GObject.Object {
    declare $signals: GObject.Object.SignalSignatures & {
        "notify::events": () => void
    }

    #timerEventsState: Timer | null = null
    #timerSourceRegistry: Timer | null = null
    #timerSources: Timer | null = null
    #timerClients: Timer | null = null

    #sourceRegistry: EDataServer.SourceRegistry | null = null
    #sources: EDataServer.Source[] = []
    #clients: ECal.Client[] = []
    #events: CalendarEvent[] = []


    #eventsState: Accessor<CalendarEvent[]>
    #setEventsState: Setter<CalendarEvent[]>
    #eventsSubscription: (() => void) | null = null

    constructor() {
        super()
        const [events, setEvents] = createState<CalendarEvent[]>([])

        this.#eventsState = events
        this.#setEventsState = setEvents

        this.#initRegistry()
        this.#updateClients()

        this.#eventsSubscription = this.#eventsState.subscribe(() => {
            this.#events = this.#eventsState.get()
            this.notify("events")
        })
    }

    @getter(Array)
    get events(): CalendarEvent[] {
        return this.#events
    }

    get sources(): EDataServer.Source[] {
        return this.#sources
    }

    get clients(): ECal.Client[] {
        return this.#clients
    }

    static get_default() {
        if (!instance) {
            instance = new Agenda()
        }
        return instance
    }

    static get_with_timer_initialized() {
        if (!instance) {
            instance = new Agenda()
            instance.initAllTimer()
        }
        return instance
    }

    initAllTimer() {
        if (!this.#sourceRegistry) {
            this.#initRegistry()
        }

        if (this.#clients.length === 0) {
            this.#updateClients()
        }

        if (this.#timerEventsState == null) {
            this.#timerEventsState = interval(5_000, () => {
                this.#setEventsState(this.#listCalendarEvents())
            })
        }

        if (this.#timerSourceRegistry == null) {
            this.#timerSourceRegistry = interval(5_000, () => {
                this.#updateSourceRegistry()
            })
        }
        if (this.#timerSources == null) {
            this.#timerSources = interval((this.#sources.length + 1) * 1000, () => {
                this.#updateSources()
            })
        }
        if (this.#timerClients == null) {
            this.#timerClients = interval((this.#clients.length + 1) * 1000, () => {
                this.#updateClients()
            })
        }
    }

    stopAllTimer() {
        this.#timerEventsState?.cancel()
        this.#timerEventsState = null;

        this.#timerSourceRegistry?.cancel()
        this.#timerSourceRegistry = null;

        this.#timerSources?.cancel()
        this.#timerSources = null;

        this.#timerClients?.cancel()
        this.#timerClients = null;

        if (this.#eventsSubscription) {
            this.#eventsSubscription()
            this.#eventsSubscription = null
        }

        this.#clients.forEach(client => {
            try {
                client.run_dispose()
            } catch (e) {
                Log.e("Agenda service", "Error disposing client", e)
            }
        })
        this.#clients = []

        if (this.#sourceRegistry) {
            try {
                this.#sourceRegistry.run_dispose()
            } catch (e) {
                Log.e("Agenda service", "Error disposing source registry", e)
            }
            this.#sourceRegistry = null
        }

        this.#sources = []
    }

    #initRegistry() {
        try {
            this.#updateSourceRegistry()
            this.#updateSources()
        } catch (e) {
            Log.e("Agenda service", `Failed to initialize registry sources or sources`, e)
            this.#sourceRegistry = null
            this.#sources = []
        }
    }

    #updateSourceRegistry() {
        if (this.#sourceRegistry) {
            try {
                this.#sourceRegistry.run_dispose()
            } catch (e) {
                Log.e("Agenda service", "Error disposing old source registry", e)
            }
        }
        this.#sourceRegistry = EDataServer.SourceRegistry.new_sync(null)
    }

    #updateSources() {
        this.#sources = this.#sourceRegistry?.list_sources(EDataServer.SOURCE_EXTENSION_CALENDAR) ?? []
    }

    #updateClients() {
        this.#clients = this.#clients
            .filter((client) => this.#sources.some((src) => client.source.uid == src.uid))
        this.#sources.forEach((source) => {
            try {
                ECal.Client.connect(
                    source,
                    ECal.ClientSourceType.EVENTS,
                    1,
                    null,
                    (client) => {
                        if (client &&
                            !this.#clients.some(cli => cli.source.uid === client.source.uid)
                        ) {
                            Log.i("Agenda service", `New client: ${client.get_source().display_name}`)
                            this.#clients = [...this.#clients, client]
                        }
                    },
                )
            } catch (e) {
                Log.e("Agenda service", `Cannot connect to client ${source.display_name}`, e)
            }
        })
    }

    #listCalendarEvents(): CalendarEvent[] {
        try {
            const now = GLib.DateTime.new_now_local()
            const start = now.add_years(-1)!
            const end = now.add_years(1)!
            const startTime = start.to_unix()
            const endTime = end.to_unix()

            const allEvents: CalendarEvent[] = []

            this.#clients.forEach((client) => {
                try {
                    const [_, comps] = client.get_object_list_sync("", null)

                    comps.forEach((comp) => {
                        client.generate_instances_for_object_sync(
                            comp,
                            startTime,
                            endTime,
                            null,
                            (generatedComp, instanceStart, instanceEnd) => {
                                const oneDay = ICalGLib.Duration.new_null_duration()
                                oneDay.set_days(1)

                                allEvents.push({
                                    summary: generatedComp.get_summary() ?? "",
                                    desc: generatedComp.get_description(),
                                    // @ts-expect-error untyped extension in EDataServer's d.ts
                                    color: client.source.get_extension(EDataServer.SOURCE_EXTENSION_CALENDAR).dup_color(),
                                    isAllDay: generatedComp.get_duration().as_ical_string() === oneDay.as_ical_string(),
                                    start: GLib.DateTime.new_from_unix_utc(instanceStart.as_timet())!,
                                    end: GLib.DateTime.new_from_unix_utc(instanceEnd.as_timet())!,
                                })
                                return true
                            },
                        )
                    })
                } catch (e) {
                    Log.e("Agenda service", `Cannot list or generate instances of an object with client ${client.source.display_name}`, e)
                }
            })

            return allEvents.toSorted((a, b) => a.start.compare(b.start))
        } catch (e) {
            Log.e("Agenda service", `Failed to list calendar events`, e)
            return []
        }
    }
}
