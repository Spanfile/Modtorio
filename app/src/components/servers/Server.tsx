import React from 'react';
import { Tabs, Tab } from 'react-bootstrap';
import { RouteComponentProps } from 'react-router';

interface IServerProps {
    serverId: string,
}

export default class Server extends React.Component<RouteComponentProps<IServerProps>> {
    public render(): JSX.Element {
        let props = this.props.match.params;
        return (
            <Tabs id="server-tab" transition={false} className="mt-3 mr-3">
                <Tab eventKey="status" title="Status">
                    Status {props.serverId}
                </Tab>
                <Tab eventKey="control" title="Control">
                    Control {props.serverId}
                </Tab>
                <Tab eventKey="logs" title="Logs">
                    Logs {props.serverId}
                </Tab>
                <Tab eventKey="config" title="Configuration">
                    Configuration {props.serverId}
                </Tab>
            </Tabs>
        );
    }
}