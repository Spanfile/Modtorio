import React from 'react';
import { Row, Col, Badge } from 'react-bootstrap';
import IServer from '../IServer';

function ModInfoRow(props: { title: string, children: React.ReactNode }) {
    return (
        <Row>
            <Col className="text-right" xs={2}>
                {props.title}
            </Col>
            <Col>
                {props.children}
            </Col>
        </Row>
    );
}

export default function (props: IServer) {
    return (
        <>
            <ModInfoRow title="Server status">Running</ModInfoRow>
            <ModInfoRow title="Uptime">1 day, 20 hours</ModInfoRow>
            <ModInfoRow title="Last autosave">2 minutes ago</ModInfoRow>
            <ModInfoRow title="Players online">1 / 8</ModInfoRow>
            <ModInfoRow title="Version">
                0.17.79 <Badge variant="success" className="rounded-0">Up-to date</Badge>
            </ModInfoRow>
        </>
    );
}
