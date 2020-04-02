import React from 'react';
import { Switch, Route, Redirect, NavLink } from 'react-router-dom';
import { Row, Col, Nav, Accordion, Card } from 'react-bootstrap';
import Server from './Server';

const baseUrl = "/servers/";

function ModNavItem(props: { serverId: string }) {
    let url = baseUrl + props.serverId;

    return (
        <Card className="rounded-0 border-0">
            <Accordion.Toggle eventKey={props.serverId} as={ModNavHeader} to={url}>
                Server {props.serverId}
            </Accordion.Toggle>
            <Accordion.Collapse eventKey={props.serverId}>
                <Card.Body className="p-0 pl-3">
                    <Nav className="flex-column">
                        <ModNavLink to={url + "/general"}>General</ModNavLink>
                        <ModNavLink to={url + "/log"}>Log</ModNavLink>
                        <ModNavLink to={url + "/configuration"}>Configuration</ModNavLink>
                    </Nav>
                </Card.Body>
            </Accordion.Collapse>
        </Card>
    );
}

function ModNavHeader(props: { to: string, children: React.ReactNode }) {
    return (
        <Card.Header as={ModNavLink} {...props} />
    );
}

function ModNavLink(props: { to: string, children: React.ReactNode }) {
    return (
        <Nav.Link as={NavLink} activeClassName="bg-primary text-white" className="text-left" {...props} />
    );
}

export default function () {
    return (
        <Row>
            <Col sm={2}>
                <Accordion className="flex-column" as={Nav} defaultActiveKey="0">
                    <ModNavItem serverId="0" />
                    <ModNavItem serverId="1" />
                </Accordion>
            </Col>

            <Col className="pl-0 mt-3 mr-3 mb-3">
                <Switch>
                    <Route path={baseUrl + ":serverId"} component={Server} />
                    <Redirect from={baseUrl} to={baseUrl + "0"} />
                </Switch>
            </Col>
        </Row >
    );
}