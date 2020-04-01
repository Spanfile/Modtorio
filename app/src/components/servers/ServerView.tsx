import React from 'react';
import { Switch, Route, Redirect, NavLink } from 'react-router-dom';
import { Row, Col, Nav } from 'react-bootstrap';
import Server from './Server';

function ModNavLink(props: { to: string, children: React.ReactNode }) {
    return (
        <Nav.Link as={NavLink} activeClassName="bg-primary text-white" className="text-left" {...props} />
    );
}

export default class ServerView extends React.Component {
    public render(): JSX.Element {
        return (
            <Row>
                <Col sm={2}>
                    <Nav className="flex-column">
                        <ModNavLink to={"/servers/0"}>Server 0</ModNavLink>
                        <ModNavLink to={"/servers/1"}>Server 1</ModNavLink>
                        <ModNavLink to={"/servers/2"}>Server 2</ModNavLink>
                        <ModNavLink to={"/servers/3"}>Server 3</ModNavLink>
                    </Nav>
                </Col>

                <Col>
                    <Switch>
                        <Route exact path="/servers" />
                        <Route path={"/servers/:serverId"} component={Server} />
                        <Redirect from="/servers" to="servers" />
                    </Switch>
                </Col>
            </Row >
        );
    }
}