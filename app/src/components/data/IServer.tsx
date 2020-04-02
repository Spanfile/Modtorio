export enum ServerState {
    NotRunning = "NOT_RUNNING",
    Starting = "STARTING",
    Running = "RUNNING",
    Saving = "SAVING",
    ShuttingDown = "SHUTTING_DOWN",
}

export interface IServer {
    id: string,
    state: ServerState,
}
