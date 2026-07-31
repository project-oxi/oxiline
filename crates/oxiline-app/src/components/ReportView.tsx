import { useTranslation } from "react-i18next";
import { useWeekReport } from "../hooks";
import type { CategoryBreakdown, RoutineStreak, WeekReport } from "../types";

const pct = (r: number | null): string => (r == null ? "—" : `${Math.round(r * 100)}%`);

/** Report tab — a dry, non-gamified weekly completion view (design spec §5).
 *  Three neutral buckets (done / skipped / no check-in) + per-category rates +
 *  per-routine current streaks. No judgment copy, no green/red verdict colors. */
export function ReportView() {
  const { t, i18n } = useTranslation();
  const { data: report, isLoading } = useWeekReport();

  const lang = i18n.language?.startsWith("en") ? "en" : "ko";
  const fmt = (s: string) => {
    const [y, m, d] = s.split("-").map(Number);
    const dt = new Date(y, m - 1, d);
    return dt.toLocaleDateString(lang === "ko" ? "ko-KR" : "en-US", {
      month: lang === "ko" ? "long" : "short",
      day: "numeric",
    });
  };
  if (isLoading || !report) {
    return (
      <div className="flex flex-1 items-center justify-center text-sm text-text-subtle">
        {t("report.loading")}
      </div>
    );
  }
  const tot = report.totals;
  return (
    <div className="flex-1 overflow-auto p-4 text-text">
      <div className="flex items-baseline justify-between">
        <h2 className="text-lg font-medium">
          {fmt(report.week_start)} – {fmt(report.week_end)}
        </h2>
        <span className="text-sm text-text-subtle">
          {t("report.thisWeek")}
        </span>
      </div>

      {/* overall rate bar — fill density only, oxide accent, no verdict hue */}
      <div className="mt-3">
        <RateBar rate={report.completion_rate} />
        <div className="mt-1 text-sm text-text-subtle">
          {t("report.rate")} {pct(report.completion_rate)}
          <span className="ml-3">
            {t("report.prevWeek")} {pct(report.prev_completion_rate)}
          </span>
        </div>
      </div>

      {/* three neutral buckets — no green/red */}
      <div className="mt-3 flex gap-5 text-sm">
        <Bucket n={tot.done} label={t("report.done")} color="var(--color-text)" />
        <Bucket n={tot.skipped} label={t("report.skipped")} color="var(--color-text-muted)" />
        <Bucket n={tot.not_recorded} label={t("report.notRecorded")} color="var(--color-text-subtle)" />
        <Bucket n={tot.upcoming} label={t("report.upcoming")} color="var(--color-text-subtle)" />
      </div>

      <Section title={t("report.categories")}>
        {report.categories.length === 0 ? (
          <Empty text={t("report.empty")} />
        ) : (
          report.categories.map((c) => <CatRow key={c.category_id ?? "none"} c={c} />)
        )}
      </Section>

      <Section title={t("report.streaks")}>
        {report.streaks.length === 0 ? (
          <Empty text={t("report.empty")} />
        ) : (
          report.streaks.map((s) => <StreakRow key={s.routine_id} s={s} unit={t("report.days")} />)
        )}
      </Section>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mt-5">
        <h3 className="mb-1 text-sm text-text-muted">
        {title}
      </h3>
      {children}
    </section>
  );
}

function Empty({ text }: { text: string }) {
  return <div className="text-sm text-text-subtle">{text}</div>;
}

function RateBar({ rate }: { rate: number | null }) {
  const w = rate == null ? 0 : Math.round(rate * 100);
  return (
    <div
      className="overflow-hidden rounded-full"
      style={{ height: 8, background: "var(--color-surface-sunken)" }}
    >
      <div
        className="h-full transition-all"
        style={{ width: `${w}%`, background: "var(--color-interactive-primary)" }}
      />
    </div>
  );
}

function Bucket({ n, label, color }: { n: number; label: string; color: string }) {
  return (
    <span style={{ color }}>
      <span className="tabular-nums">{n}</span>{" "}
      <span style={{ color: "var(--color-text-subtle)" }}>{label}</span>
    </span>
  );
}

function CatRow({ c }: { c: CategoryBreakdown }) {
  const denom = c.done + c.not_recorded;
  const w = c.completion_rate == null ? 0 : Math.round(c.completion_rate * 100);
  return (
    <div className="py-1">
      <div className="flex justify-between text-sm">
        <span>{c.category_name || "—"}</span>
        <span className="tabular-nums text-text-subtle">
          {c.done}/{denom} · {pct(c.completion_rate)}
        </span>
      </div>
      <div
        className="mt-0.5 overflow-hidden rounded-full"
        style={{ height: 6, background: "var(--color-surface-sunken)" }}
      >
        <div className="h-full" style={{ width: `${w}%`, background: "var(--color-interactive-primary)" }} />
      </div>
    </div>
  );
}

function StreakRow({ s, unit }: { s: RoutineStreak; unit: string }) {
  return (
    <div className="flex justify-between py-0.5 text-sm">
      <span>{s.title}</span>
      <span className="tabular-nums text-text-subtle">
        {s.current}
        {unit}
      </span>
    </div>
  );
}
