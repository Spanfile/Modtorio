import React from 'react';
import { Table, DropdownButton, Dropdown } from 'react-bootstrap';
import IServer from '../IServer';

function Player(props: { name: string, role: string, online: boolean, last_seen: string }) {
    return (
        <tr>
            <td className="align-middle">{props.name}</td>
            <td className="align-middle">{props.role}</td>
            <td className="align-middle">{props.online}</td>
            <td className="align-middle">{props.last_seen}</td>
            <td>
                <DropdownButton id={props.name + "-control"} title="Control" size="sm" alignRight>
                    <Dropdown.Item as="button">Kick</Dropdown.Item>
                    <Dropdown.Item as="button">Ban</Dropdown.Item>
                    <Dropdown.Divider />
                    <Dropdown.Header>Chat</Dropdown.Header>
                    <Dropdown.Item as="button">Mute</Dropdown.Item>
                    <Dropdown.Item as="button" disabled>Unmute</Dropdown.Item>
                    <Dropdown.Item as="button">Purge</Dropdown.Item>
                    <Dropdown.Divider />
                    <Dropdown.Header>Administrator</Dropdown.Header>
                    <Dropdown.Item as="button">Promote</Dropdown.Item>
                    <Dropdown.Item as="button" disabled>Demote</Dropdown.Item>
                </DropdownButton>
            </td>
        </tr>
    );
}

export default function (props: IServer) {
    return (
        <Table striped bordered hover size="sm">
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
                <Player name="Spans" role="Administrator" online={true} last_seen="1 day ago" />
                <Player name="Whitensnake" role="Player" online={true} last_seen="1 day ago" />
                <Player name="nuppih" role="Player" online={true} last_seen="1 year ago" />
            </tbody>
        </Table>
    );
}
