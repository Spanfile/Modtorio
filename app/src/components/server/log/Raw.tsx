import React from 'react';
import { Table } from 'react-bootstrap';
import IServer from '../IServer';

export default class Raw extends React.Component<IServer> {
    public render(): JSX.Element {
        return (
            <Table striped bordered size="sm" className="m-3">
                <thead>
                    <tr>
                        <th>Time</th>
                        <th>Severity</th>
                        <th>Line</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>1</td>
                        <td>Info</td>
                        <td>AppManager.cpp:394: Saving game as /opt/factorio/saves/top_vitun_kek.zip</td>
                    </tr>
                    <tr>
                        <td>2</td>
                        <td>Info</td>
                        <td>AppManager.cpp:397: Saving finished</td>
                    </tr>
                </tbody>
            </Table>
        );
    }
}