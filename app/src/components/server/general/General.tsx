import React from 'react';
import { Card } from 'react-bootstrap';
import IServerProps from '../IServerProps';
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

export default class General extends React.Component<IServerProps> {
    public render(): JSX.Element {
        return (
            <>
                <ModCard title="Control">
                    <Control {...this.props} />
                </ModCard>
                <ModCard title="Status">
                    <Status {...this.props} />
                </ModCard>
                <ModCard title="Players">
                    <Players {...this.props} />
                </ModCard>
            </>
        );
    }
}