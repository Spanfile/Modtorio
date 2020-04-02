import React from 'react';
import { Tabs, Tab } from 'react-bootstrap';
import { Parsed } from './Parsed';
import { Raw } from './Raw';

export function Log(props: { id: string | undefined }) {
    return (
        <Tabs defaultActiveKey="parsed" transition={false} id="log-tab">
            <Tab eventKey="parsed" title="Parsed">
                <Parsed {...props} />
            </Tab>
            <Tab eventKey="raw" title="Raw">
                <Raw {...props} />
            </Tab>
        </Tabs>
    );
}