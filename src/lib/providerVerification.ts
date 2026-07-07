const VERIFIED_KEY = "dle.verifiedProviderIds";
const LAST_VERIFIED_KEY = "dle.lastVerifiedProviderId";

function readVerifiedIds(): string[] {
  try {
    const raw = localStorage.getItem(VERIFIED_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === "string") : [];
  } catch {
    return [];
  }
}

function writeVerifiedIds(ids: string[]): void {
  try {
    localStorage.setItem(VERIFIED_KEY, JSON.stringify(ids));
  } catch {
    // Verification state is a UX convenience; ignore storage failures.
  }
}

export function markProviderVerified(providerId: string): void {
  const ids = readVerifiedIds();
  if (!ids.includes(providerId)) {
    ids.push(providerId);
  }
  writeVerifiedIds(ids);
  try {
    localStorage.setItem(LAST_VERIFIED_KEY, providerId);
  } catch {
    // ignore
  }
}

export function clearProviderVerification(providerId: string): void {
  writeVerifiedIds(readVerifiedIds().filter((id) => id !== providerId));
  try {
    if (localStorage.getItem(LAST_VERIFIED_KEY) === providerId) {
      localStorage.removeItem(LAST_VERIFIED_KEY);
    }
  } catch {
    // ignore
  }
}

export function isProviderVerified(providerId: string): boolean {
  return readVerifiedIds().includes(providerId);
}

export function lastVerifiedProviderId(): string | null {
  try {
    return localStorage.getItem(LAST_VERIFIED_KEY);
  } catch {
    return null;
  }
}

const SELECTED_KEY = "dle.selectedProviderId";

/** The model chosen on the analysis page; also used by the workbench chat. */
export function setSelectedProviderId(providerId: string): void {
  try {
    localStorage.setItem(SELECTED_KEY, providerId);
  } catch {
    // ignore
  }
}

export function selectedProviderId(): string | null {
  try {
    return localStorage.getItem(SELECTED_KEY);
  } catch {
    return null;
  }
}
