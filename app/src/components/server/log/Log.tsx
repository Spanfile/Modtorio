import React from 'react';
import { Tabs, Tab } from 'react-bootstrap';
import IServer from '../IServer';
import Parsed from './Parsed';
import Raw from './Raw';

export default class Log extends React.Component<IServer> {
    public render(): JSX.Element {
        return (
            <Tabs defaultActiveKey="parsed" transition={false} id="log-tab">
                <Tab eventKey="parsed" title="Parsed">
                    <Parsed {...this.props} />
                </Tab>
                <Tab eventKey="raw" title="Raw">
                    <Raw {...this.props} />
                </Tab>
            </Tabs>
        );
    }
}