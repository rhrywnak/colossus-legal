export type StatusResponse = {
    app: string;
    version: string;
    status: string;
};

export async function getStatus(): Promise<StatusResponse> {
    const baseUrl = import.meta.env.VITE_API_URL || "http://localhost:3403";
    const response = await fetch(`${baseUrl}/api/status`);

    if (!response.ok) {
        throw new Error(`Status request failed with ${response.status}`);
    }

    let data: unknown;
    try {
        data = await response.json();
    } catch (error) {
        throw new Error("Failed to parse status response");
    }

    const parsed = data as Partial<StatusResponse>;
    if (!parsed.app || !parsed.version || !parsed.status) {
        throw new Error("Invalid status response shape");
    }

    return {
        app: parsed.app,
        version: parsed.version,
        status: parsed.status,
    };
}
