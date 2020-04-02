import React from 'react';
import { Card } from 'react-bootstrap';
import IServer from '../IServer';
import Control from './Control';
import Status from './Status';
import Players from './Players';

function ModCard(props: { title: string, children: React.ReactNode }) {
    return (
        <Card className="mb-3">
            <Card.Header className="bg-secondary text-white">{props.title}</Card.Header>
            <Card.Body>{props.children}</Card.Body>
        </Card>
    );
}

export default function (props: IServer) {
    return (
        <>
            <ModCard title="Control">
                <Control {...props} />
            </ModCard>
            <ModCard title="Status">
                <Status {...props} />
            </ModCard>
            <ModCard title="Players">
                <Players {...props} />
            </ModCard>
        </>
    );
}
