import React from 'react';
import { Table } from 'react-bootstrap';
import IServerProps from '../IServerProps';

export default class Status extends React.Component<IServerProps> {
    public render(): JSX.Element {
        return (
            <Table striped bordered hover>
                <thead>
                    <tr>
                        <th>Name</th>
                        <th>Role</th>
                        <th>Online</th>
                        <th>Last seen</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>Spans</td>
                        <td>Administrator</td>
                        <td>Yes</td>
                        <td>1 day ago</td>
                        <td></td>
                    </tr>
                    <tr>
                        <td>whitensnake</td>
                        <td>Player</td>
                        <td>No</td>
                        <td>1 day ago</td>
                        <td></td>
                    </tr>
                    <tr>
                        <td>some gui</td>
                        <td>Player</td>
                        <td>No</td>
                        <td>1 day ago</td>
                        <td></td>
                    </tr>
                </tbody>
            </Table>
        );
    }
}