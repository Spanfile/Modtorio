import React from 'react';
import { Table } from 'react-bootstrap';

export function Parsed(props: { id: string | undefined }) {
    return (
        <Table striped bordered size="sm" className="m-3">
            <thead>
                <tr>
                    <th>Time</th>
                    <th>Severity</th>
                    <th>Event</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>1</td>
                    <td>Info</td>
                    <td>Player <strong>Spans</strong> joined</td>
                </tr>
                <tr>
                    <td>2</td>
                    <td>Info</td>
                    <td>Map saved to <strong>_autosave1.zip</strong></td>
                </tr>
            </tbody>
        </Table>
    );
}