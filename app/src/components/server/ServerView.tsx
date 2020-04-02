import React from 'react';
import { Switch, Route, Redirect, NavLink, useRouteMatch } from 'react-router-dom';
import { Row, Col, Nav, Accordion, Card } from 'react-bootstrap';
import Server from './Server';

function ModNavItem(props: { id: string, path: string }) {
    let url = props.path + "/" + props.id;

    return (
        <Card className="rounded-0 border-0">
            <Accordion.Toggle eventKey={props.id} as={ModNavHeader} to={url}>
                Server {props.id}
            </Accordion.Toggle>
            <Accordion.Collapse eventKey={props.id}>
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

export function ServerView() {
    let match = useRouteMatch();

    return (
        <Row>
            <Col sm={2}>
                <Accordion className="flex-column" as={Nav} defaultActiveKey="0">
                    <ModNavItem id="0" path={match.url} />
                    <ModNavItem id="1" path={match.url} />
                </Accordion>
            </Col>

            <Col className="pl-0 mt-3 mr-3 mb-3">
                <Switch>
                    <Route path={`${match.path}/:id`} component={Server} />
                    <Redirect from={match.path} to={`${match.path}/0`} />
                </Switch>
            </Col>
        </Row >
    );
}