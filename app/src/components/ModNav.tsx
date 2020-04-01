import React from 'react';
import { Row, Col, Navbar, Nav } from 'react-bootstrap';
import { Link } from 'react-router-dom';

export default class ModNav extends React.Component {
    public render(): JSX.Element {
        return (
            <Row>
                <Col>
                    <Navbar bg="dark" variant="dark">
                        <Navbar.Brand as={Link} to="/servers">Modtorio</Navbar.Brand>
                        <Nav className="mr-auto">
                            <Nav.Link as={Link} to="/servers">Servers</Nav.Link>
                            <Nav.Link as={Link} to="/settings">Settings</Nav.Link>
                        </Nav>
                    </Navbar>
                </Col>
            </Row>
        );
    }
}