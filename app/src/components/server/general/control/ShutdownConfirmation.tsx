import React from 'react';
import { StepComponentProps } from 'components/common/Wizard';

type ShutdownConfirmationProps = {
    playerCount: number,
}

export function ShutdownConfirmation(props: StepComponentProps<ShutdownConfirmationProps>) {
    return (
        <>
            <p>Are you sure you want to shut down the server?</p>
            {props.playerCount > 0 ? (
                <p>There are {props.playerCount} players online. They will be notified and the server will be shut down after they've left, or after a timeout has passed.</p>
            )
                : (
                    <p>There are no players online, so the server can be shut down immediately.</p>
                )}
        </>
    );
}
