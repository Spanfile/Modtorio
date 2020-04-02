import React from 'react';
import { Modal } from 'react-bootstrap';

type ModModalVariant =
    "light" |
    "primary" |
    "succcess" |
    "danger";

export interface ModModalProps {
    title: string,
    onHide: any,
    show: boolean,
    variant?: ModModalVariant,
    children: React.ReactNode,
}

const variantToText: { [bg: string]: string } = {
    "bg-light": "text-dark",
    "bg-primary": "text-dark",
    "bg-success": "text-dark",
    "bg-danger": "text-white",
};

export function ServerControlModal(props: ModModalProps) {
    let bg = "bg-light";

    if (props.variant != null) {
        bg = `bg-${props.variant}`;
    }

    let text = variantToText[bg];

    return (
        <Modal
            {...props}
            centered
        >
            <Modal.Header className={`${bg} ${text}`}>
                <Modal.Title>{props.title}</Modal.Title>
            </Modal.Header>
            <Modal.Body>
                {props.children}
            </Modal.Body>
        </Modal >
    );
}
