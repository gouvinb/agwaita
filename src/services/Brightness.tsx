import {monitorFile, readFile, readFileAsync} from "ags/file"
import GObject, {getter, register, setter} from "gnim/gobject"
import GLib from "gi://GLib"
import DesktopScriptLib, {bash} from "../lib/ExternalCommand"
import {Log} from "../lib/Logger";

const screenDevice = bash(`ls -w1 /sys/class/backlight | head -1`)
const screen = `/sys/class/backlight/${screenDevice}`

let instance: Brightness

@register({GTypeName: "Brightness"})
export default class Brightness extends GObject.Object {
    declare $signals: GObject.Object.SignalSignatures & {
        "notify::screen": () => void
        "notify::kbd": () => void
    }
    #screenMax = Number(screenDevice ? readFile(`${screen}/max_brightness`) : "0")
    #screen = Number(screenDevice ? readFile(`${screen}/brightness`) : "0") / this.#screenMax


    #pendingValue: number | null = null
    #debounceId: number | null = null
    #debounceDelay = 33 // ms

    constructor() {
        super()

        monitorFile(`${screen}/brightness`, async (f) => {
            const v = await readFileAsync(f)
            this.#screen = Number(v) / this.#screenMax
            this.notify("screen")
        })
    }

    @getter(Number)
    get screen() {
        return this.#screen
    }

    @setter(Number)
    set screen(percent) {
        if (percent < 0) percent = 0
        if (percent > 1) percent = 1

        this.#pendingValue = percent

        if (this.#debounceId) {
            GLib.source_remove(this.#debounceId)
        }

        this.#debounceId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, this.#debounceDelay, () => {
            this.#applyBrightness(this.#pendingValue!)
            this.#debounceId = null
            return GLib.SOURCE_REMOVE
        })
    }

    static get_default() {
        if (!instance) instance = new Brightness()
        return instance
    }

    async #applyBrightness(percent: number) {
        try {
            await DesktopScriptLib.execAsync(`light set ${Math.floor(percent * 100)}`)
            this.#screen = percent
            this.notify("screen")
        } catch (error) {
            Log.e("Brightness service", `Cannot apply ${percent}%`, error);
        }
    }
}

