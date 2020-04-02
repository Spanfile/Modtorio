import React from 'react';
import { StepComponentProps } from 'components/common/Wizard';

export type WaitPlayersProps = {
    timeout: number,
}

export function WaitPlayers(props: StepComponentProps<WaitPlayersProps>) {
    const [countdown, setCountdown] = React.useState(props.timeout);
    const { next } = props;

    React.useEffect(() => {
        const timeout = setTimeout(() => setCountdown(countdown - 1), 1000);

        if (countdown === 0) {
            next();
        }

        return () => clearTimeout(timeout);
    }, [countdown, next]);

    return (
        <>
            <p>Waiting for players to leave: 1 remaining</p>
            <p>{countdown}</p>
        </>
    );
}
