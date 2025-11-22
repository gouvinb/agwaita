import GLib from "gi://GLib"

export const DateTimeExt = {
    isSameDate(a: GLib.DateTime, b: GLib.DateTime): boolean {
        const fa = a.format("%Y%m%d")
        const fb = b.format("%Y%m%d")
        return fa !== null && fb !== null && fa === fb
    },

    isToday(dt: GLib.DateTime): boolean {
        return DateTimeExt.isSameDate(dt, GLib.DateTime.new_now_local())
    },
};
