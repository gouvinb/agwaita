import {createPoll, interval} from "ags/time"
import GLib from "gi://GLib"
import EDataServer from "gi://EDataServer"
import ECal from "gi://ECal"
import ICalGLib from "gi://ICalGLib"
import GObject, {getter, register} from "gnim/gobject"
import {Log} from "../lib/Logger";

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

    #sourceRegistry: EDataServer.SourceRegistry | null = null
    #sources: EDataServer.Source[] = []
    #clients: ECal.Client[] = []
    #events: CalendarEvent[] = []

    #eventsPoll = createPoll<CalendarEvent[]>(
        [],
        1000,
        () => this.#listCalendarEvents(),
    )

    constructor() {
        super()
        this.#initRegistry()
        this.#updateClients()

        this.#eventsPoll.subscribe(() => {
            this.#events = this.#eventsPoll.get()
            this.notify("events")
        })

        interval(5_000, () => {
            this.#updateSourceRegistry()
        })
        interval((this.#sources.length + 1) * 1000, () => {
            this.#updateSources()
        })
        interval((this.#clients.length + 1) * 1000, () => {
            this.#updateClients()
        })
    }

    static get_default() {
        if (!instance) instance = new Agenda()
        return instance
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
                                    // @ts-expect-error extension non typée dans les d.ts de EDataServer
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
