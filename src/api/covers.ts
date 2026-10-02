import { coverBytes } from "./library";

// Only two visible-cover jobs decode concurrently; reading has its own cache.
let active = 0;
const waiting: (() => void)[] = [];
function drain() {
  while (active < 2 && waiting.length) waiting.shift()!();
}
export function loadCover(
  id: number,
  cancelled: () => boolean,
): Promise<string | null> {
  return new Promise((resolve, reject) => {
    waiting.push(() => {
      if (cancelled()) {
        resolve(null);
        drain();
        return;
      }
      active++;
      coverBytes(id)
        .then((bytes) => {
          resolve(
            cancelled()
              ? null
              : URL.createObjectURL(new Blob([bytes], { type: "image/png" })),
          );
        }, reject)
        .finally(() => {
          active--;
          drain();
        });
    });
    drain();
  });
}
