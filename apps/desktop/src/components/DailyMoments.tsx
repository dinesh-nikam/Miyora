import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invokeSafe } from "../utils/ipc";

/** Mirrors `agentpet_core::DailyMoment`. */
export interface DailyMoment {
  petId: string;
  localDate: string;
  messageId: string;
  text: string;
  createdAt: string;
}

const DAILY_MOMENT_EVENT = "daily-moment";

/** History of the once-a-day companion moments recorded by Rust. */
export function DailyMoments({ active }: { active: boolean }) {
  const [moments, setMoments] = useState<DailyMoment[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!active) return;
    let stop: (() => void) | undefined;
    let disposed = false;
    const load = () => {
      void invokeSafe<DailyMoment[]>("get_daily_moments").then((result) => {
        if (disposed) return;
        if (result.ok) {
          setMoments(result.data);
          setError(null);
        } else {
          setError(result.error);
        }
      });
    };
    load();
    listen(DAILY_MOMENT_EVENT, load)
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch((reason: unknown) => console.error("[DailyMoments] listener unavailable:", reason));
    return () => {
      disposed = true;
      stop?.();
    };
  }, [active]);

  if (error) return <p className="deck-quiet">Daily moments are unavailable: {error}</p>;
  if (moments.length === 0) {
    return <p className="deck-quiet">Your companion shares one gentle moment a day, after 9:00.</p>;
  }
  return (
    <ul className="deck-moments" aria-label="Daily companion moments">
      {moments.map((moment) => (
        <li key={`${moment.petId}-${moment.localDate}`}>
          <time dateTime={moment.localDate} className="deck-quiet">
            {moment.localDate}
          </time>{" "}
          {moment.text}
        </li>
      ))}
    </ul>
  );
}
