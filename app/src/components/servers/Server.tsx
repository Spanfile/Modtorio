import React from 'react';
import { Nav } from 'react-bootstrap';
import { RouteComponentProps } from 'react-router';

interface IServerProps {
    serverId: string,
}

export default class Server extends React.Component<RouteComponentProps<IServerProps>> {
    public render(): JSX.Element {
        let props = this.props.match.params;
        return (
            <Nav variant="tabs">

            </Nav>
        );
    }
}