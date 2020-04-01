import React from 'react';
import { Switch, Route } from 'react-router-dom';
import { Row, Col, Nav } from 'react-bootstrap';

export default class ServerView extends React.Component {
    public render(): JSX.Element {
        return (
            <Row>
                <Col sm={2}>
                    <Nav>
                        <Nav.Link href={"/servers/0"}>Server 0</Nav.Link>
                    </Nav>
                </Col>

                <Col>
                    <Switch>
                        <Route exact={true} path={"/servers"} />
                        <Route path={"/servers/:serverId"} />
                    </Switch>
                </Col>
            </Row>
        );
    }
}